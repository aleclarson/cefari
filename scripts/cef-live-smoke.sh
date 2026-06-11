#!/usr/bin/env bash
set -euo pipefail

if [[ "${CEFARI_LIVE_CEF_SMOKE:-}" != "1" ]]; then
  echo "skipping live CEF smoke; set CEFARI_LIVE_CEF_SMOKE=1 to run it"
  exit 0
fi

if [[ -z "${CEFARI_CEF_RESOURCES_DIR:-}" ]]; then
  echo "CEFARI_CEF_RESOURCES_DIR must point at extracted CEF resources" >&2
  exit 2
fi

if [[ ! -f "$CEFARI_CEF_RESOURCES_DIR/archive.json" ]]; then
  echo "CEFARI_CEF_RESOURCES_DIR is missing archive.json: $CEFARI_CEF_RESOURCES_DIR" >&2
  exit 2
fi

if [[ ! -d "$CEFARI_CEF_RESOURCES_DIR/locales" ]]; then
  echo "CEFARI_CEF_RESOURCES_DIR is missing locales/: $CEFARI_CEF_RESOURCES_DIR" >&2
  exit 2
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
smoke_root="${CEFARI_SMOKE_ROOT:-"$repo_root/.tmp/cef-live-smoke"}"
resource_root="$smoke_root/resources"
frontend_dir="$resource_root/frontend"
stdout_log="$smoke_root/cefari-desktop.stdout.log"
stderr_log="$smoke_root/cefari-desktop.stderr.log"
exit_after_ms="${CEFARI_SMOKE_EXIT_AFTER_MS:-8000}"
watchdog_seconds="${CEFARI_SMOKE_WATCHDOG_SECONDS:-20}"

rm -rf "$smoke_root"
mkdir -p "$frontend_dir"

cat > "$frontend_dir/index.html" <<'HTML'
<!doctype html>
<html lang="en">
<meta charset="utf-8">
<title>Cefari Smoke Pending</title>
<style>
  body { font: 15px/1.5 system-ui, sans-serif; margin: 32px; color: #17202a; }
  pre { background: #f4f6f8; border: 1px solid #d5dbe3; padding: 12px; white-space: pre-wrap; }
</style>
<h1>Cefari CEF Smoke</h1>
<pre id="status">loading</pre>
<script>
const status = document.getElementById("status");
const lines = [];
const report = (line) => {
  lines.push(`${new Date().toISOString()} ${line}`);
  status.textContent = lines.join("\n");
};

async function invoke(command) {
  report(`invoke ${command.command}`);
  const response = await window.cefari.invoke(command);
  report(`${command.command} -> ${response.outcome.status}`);
  if (response.outcome.status !== "ok") {
    throw new Error(`${command.command} failed: ${JSON.stringify(response.outcome.payload)}`);
  }
  return response.outcome.payload;
}

async function runSmoke() {
  report(`location ${location.href}`);
  if (!location.href.startsWith("cefari://app/")) {
    throw new Error(`expected cefari://app resource, got ${location.href}`);
  }
  if (!window.cefari || typeof window.cefari.invoke !== "function") {
    throw new Error("window.cefari bridge is unavailable");
  }
  report("bridge available");
  await invoke({ command: "updateState" });

  if (sessionStorage.getItem("cefariSmokeReloaded") !== "1") {
    sessionStorage.setItem("cefariSmokeReloaded", "1");
    await invoke({ command: "reloadUi" });
    report("reload requested");
    return;
  }

  report("reload observed");
  await invoke({ command: "windowSetTitle", payload: { title: "Cefari Smoke PASS" } });
  document.title = "Cefari Smoke PASS";
  report("pass");
}

runSmoke().catch((error) => {
  document.title = "Cefari Smoke FAIL";
  report(`fail ${error && error.message ? error.message : error}`);
});
</script>
</html>
HTML

echo "building cefari-desktop"
cargo build -p cefari-desktop

desktop_bin="$repo_root/target/debug/cefari-desktop"
if [[ ! -x "$desktop_bin" ]]; then
  echo "missing desktop binary after build: $desktop_bin" >&2
  exit 2
fi

echo "running live CEF smoke"
echo "  UI resources: $resource_root"
echo "  CEF resources: $CEFARI_CEF_RESOURCES_DIR"
echo "  stdout: $stdout_log"
echo "  stderr: $stderr_log"

set +e
if command -v timeout >/dev/null 2>&1; then
  CEFARI_RESOURCE_DIR="$resource_root" \
  CEFARI_SMOKE_EXIT_AFTER_MS="$exit_after_ms" \
  timeout "$watchdog_seconds" "$desktop_bin" >"$stdout_log" 2>"$stderr_log"
  status=$?
else
  CEFARI_RESOURCE_DIR="$resource_root" \
  CEFARI_SMOKE_EXIT_AFTER_MS="$exit_after_ms" \
  "$desktop_bin" >"$stdout_log" 2>"$stderr_log" &
  pid=$!
  (
    sleep "$watchdog_seconds"
    kill "$pid" >/dev/null 2>&1
  ) &
  watchdog=$!
  wait "$pid"
  status=$?
  kill "$watchdog" >/dev/null 2>&1
fi
set -e

if [[ "$status" -ne 0 ]]; then
  echo "live CEF smoke failed with exit status $status" >&2
  echo "see $stdout_log and $stderr_log" >&2
  exit "$status"
fi

echo "live CEF smoke completed; confirm the window showed 'Cefari Smoke PASS' when running interactively"
