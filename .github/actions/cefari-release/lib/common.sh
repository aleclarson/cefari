# shellcheck shell=bash
# shellcheck disable=SC2154

fail() {
  echo "cefari-release: $*" >&2
  exit 1
}

bool_input() {
  case "$1" in
    true|false) return 0 ;;
    *) return 1 ;;
  esac
}

quote_args() {
  printf "%q" "$1"
  shift
  for arg in "$@"; do
    printf " %q" "$arg"
  done
}

run_cmd() {
  printf "+ "
  quote_args "$@"
  printf "\n"
  if [[ "$dry_run" != "true" ]]; then
    "$@"
  fi
}

command_available() {
  command -v "$1" >/dev/null 2>&1
}

validate_command_available() {
  local command_name="$1"
  if [[ "$dry_run" == "true" ]]; then
    return 0
  fi
  command_available "$command_name" || fail "$command_name is required but was not found"
}
