#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
smoke_dir="$repo_root/smoke"
run_root="${CEFARI_SMOKE_RUN_ROOT:-"$repo_root/.tmp/cefari-dev-smoke"}"
home_dir="$run_root/home"
stdout_log="$run_root/cefari-dev.stdout.log"
stderr_log="$run_root/cefari-dev.stderr.log"
frontmost_log="$run_root/frontmost.log"
frontmost_fail="$run_root/frontmost.fail"
devtools_probe="$run_root/devtools-version.json"
watchdog_seconds="${CEFARI_SMOKE_WATCHDOG_SECONDS:-60}"
exit_after_ms="${CEFARI_SMOKE_EXIT_AFTER_MS:-45000}"
vite_port="${CEFARI_SMOKE_VITE_PORT:-5273}"
original_home="${HOME:-}"

frontmost_app() {
  osascript -e 'tell application "System Events" to name of first application process whose frontmost is true' 2>/dev/null || true
}

monitor_frontmost() {
  local app
  while true; do
    app="$(frontmost_app)"
    if [[ -n "$app" ]]; then
      printf '%s %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$app" >> "$frontmost_log"
      case "$app" in
        Cefari|Cefari\ Dev|Cefari\ Smoke|cefari|cefari-desktop)
          printf '%s\n' "$app" > "$frontmost_fail"
          ;;
      esac
    fi
    sleep 0.5
  done
}

rm -rf "$run_root"
rm -rf "$smoke_dir/.cefari"
mkdir -p "$home_dir"

echo "building cefari binaries"
pnpm --dir npm build
cargo build -p cefari-desktop

if [[ ! -f "$repo_root/npm/dist/bin/cefari.js" ]]; then
  echo "missing cefari CLI binary at $repo_root/npm/dist/bin/cefari.js" >&2
  exit 2
fi

if [[ -n "$(frontmost_app)" ]]; then
  monitor_frontmost &
  monitor_pid=$!
else
  monitor_pid=""
  echo "frontmost app sampling unavailable; continuing without focus sampling" >> "$frontmost_log"
fi

cleanup() {
  if [[ -n "${monitor_pid:-}" ]]; then
    kill "$monitor_pid" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

echo "running cefari dev smoke"
echo "  stdout: $stdout_log"
echo "  stderr: $stderr_log"

set +e
env \
  HOME="$home_dir" \
  CARGO_HOME="${CARGO_HOME:-"$original_home/.cargo"}" \
  RUSTUP_HOME="${RUSTUP_HOME:-"$original_home/.rustup"}" \
  DENO_DIR="$run_root/deno" \
  CEFARI_SMOKE_BACKGROUND=1 \
  CEFARI_SMOKE_EXIT_AFTER_MS="$exit_after_ms" \
  node "$repo_root/npm/dist/bin/cefari.js" dev "$smoke_dir" --vite-port "$vite_port" \
  >"$stdout_log" 2>"$stderr_log" &
smoke_pid=$!

status=0
devtools_verified=0
deadline=$((SECONDS + watchdog_seconds))
while kill -0 "$smoke_pid" >/dev/null 2>&1; do
  if [[ "$devtools_verified" -eq 0 && -f "$smoke_dir/.cefari/devtools.json" ]]; then
    devtools_url="$(sed -n 's/.*"browserUrl"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$smoke_dir/.cefari/devtools.json" | head -1)"
    if [[ -n "$devtools_url" ]] && curl -fsS "$devtools_url/json/version" >"$devtools_probe" 2>/dev/null; then
      devtools_verified=1
    fi
  fi
  if (( SECONDS >= deadline )); then
    echo "cefari dev smoke timed out after ${watchdog_seconds}s" >&2
    kill "$smoke_pid" >/dev/null 2>&1 || true
    status=124
    break
  fi
  sleep 0.1
done

if [[ "$status" -eq 0 ]]; then
  wait "$smoke_pid"
  status=$?
else
  wait "$smoke_pid" >/dev/null 2>&1 || true
fi
set -e

cleanup
trap - EXIT

if [[ "$status" -ne 0 ]]; then
  echo "cefari dev smoke failed with status $status" >&2
  echo "see $stdout_log and $stderr_log" >&2
  exit "$status"
fi

if [[ "$devtools_verified" -ne 1 ]]; then
  echo "CEF DevTools Protocol endpoint did not respond during smoke" >&2
  echo "see $smoke_dir/.cefari/devtools.json and $devtools_probe" >&2
  exit 1
fi

if [[ -f "$frontmost_fail" ]]; then
  echo "smoke brought Cefari to the front: $(cat "$frontmost_fail")" >&2
  echo "see $frontmost_log" >&2
  exit 1
fi

result_file="$(find "$home_dir" -path '*/smoke/result.json' -type f -print -quit)"
if [[ -z "$result_file" ]]; then
  echo "smoke result file was not written under $home_dir" >&2
  echo "see $stdout_log and $stderr_log" >&2
  exit 1
fi

if ! grep -q '"status": "pass"' "$result_file"; then
  echo "smoke result did not pass: $result_file" >&2
  cat "$result_file" >&2
  exit 1
fi

echo "cefari dev smoke passed"
echo "  result: $result_file"
echo "  stdout: $stdout_log"
echo "  stderr: $stderr_log"
