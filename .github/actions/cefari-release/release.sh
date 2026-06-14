#!/usr/bin/env bash
set -euo pipefail

project_path="${CEFARI_PROJECT_PATH:-.}"
mode="${CEFARI_RELEASE_MODE:-release}"
targets="${CEFARI_TARGETS:-}"
cefari_command="${CEFARI_COMMAND:-cefari}"
install_cli="${CEFARI_INSTALL_CLI:-false}"
cefari_version="${CEFARI_CLI_VERSION:-latest}"
version="${CEFARI_RELEASE_VERSION:-}"
release_tag="${CEFARI_RELEASE_TAG:-}"
release_name="${CEFARI_RELEASE_NAME:-}"
create_github_release="${CEFARI_CREATE_GITHUB_RELEASE:-true}"
signing_platform="${CEFARI_SIGNING_PLATFORM:-}"
signing_config="${CEFARI_SIGNING_CONFIG:-}"
notarize="${CEFARI_NOTARIZE:-false}"
update_url_base="${CEFARI_UPDATE_URL_BASE:-}"
update_target="${CEFARI_UPDATE_TARGET:-}"
update_format="${CEFARI_UPDATE_FORMAT:-}"
update_key_env="${CEFARI_UPDATE_KEY_ENV:-UPDATE_SIGNING_KEY}"
dry_run="${CEFARI_DRY_RUN:-false}"

package_dir="$project_path/dist/package"
update_dir="$project_path/dist/update"
artifact_dir="$project_path/dist"

write_outputs() {
  {
    echo "package-dir=$package_dir"
    echo "update-dir=$update_dir"
    echo "artifact-dir=$artifact_dir"
    echo "release-mode=$mode"
  } >> "$GITHUB_OUTPUT"
}

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

find_release_asset() {
  find "$package_dir/output" -type f \
    \( -name '*.dmg' -o -name '*.app.tar.gz' -o -name '*.AppImage' -o -name '*.deb' -o -name '*.rpm' -o -name '*.exe' -o -name '*.msi' -o -name '*.zip' -o -name '*.tar.gz' \) \
    | sort \
    | head -n 1
}

[[ "$mode" == "release" || "$mode" == "prerelease" ]] || fail "mode must be release or prerelease"
bool_input "$create_github_release" || fail "create-github-release must be true or false"
bool_input "$install_cli" || fail "install-cli must be true or false"
bool_input "$notarize" || fail "notarize must be true or false"
bool_input "$dry_run" || fail "dry-run must be true or false"
[[ -n "$cefari_command" ]] || fail "cefari-command is required"
[[ "$install_cli" != "true" || -n "$cefari_version" ]] || fail "cefari-version is required when install-cli is true"
[[ -n "$version" ]] || fail "release-version is required"
[[ -f "$project_path/cefari.toml" ]] || fail "cefari.toml not found at $project_path"

write_outputs

echo "Cefari release plan"
echo "  project: $project_path"
echo "  mode: $mode"
echo "  version: $version"
echo "  targets: ${targets:-current runner}"
echo "  cefari command: $cefari_command"
echo "  install cli: $install_cli"
echo "  dry-run: $dry_run"

if [[ "$install_cli" == "true" ]]; then
  validate_command_available npm
  run_cmd npm install -g "@cefari/cli@$cefari_version"
fi

validate_command_available "$cefari_command"

run_cmd "$cefari_command" build "$project_path" --release
run_cmd "$cefari_command" package "$project_path" --release

asset=""
if [[ "$dry_run" != "true" ]]; then
  asset="$(find_release_asset || true)"
  [[ -n "$asset" ]] || fail "no release asset found under $package_dir/output"
  echo "selected release asset: $asset"
fi

if [[ -n "$signing_config" || -n "$signing_platform" ]]; then
  [[ "$dry_run" == "true" || -n "$asset" ]] || fail "cannot sign without a release asset"
  sign_args=("$cefari_command" codesign "$asset")
  [[ -n "$signing_platform" ]] && sign_args+=(--platform "$signing_platform")
  [[ -n "$signing_config" ]] && sign_args+=(--config "$signing_config")
  run_cmd "${sign_args[@]}"
else
  echo "signing skipped: no signing platform or signing config provided"
fi

if [[ "$notarize" == "true" ]]; then
  [[ "$dry_run" == "true" || -n "$asset" ]] || fail "cannot notarize without a release asset"
  notarize_args=("$cefari_command" notarize "$asset")
  [[ -n "$signing_config" ]] && notarize_args+=(--config "$signing_config")
  run_cmd "${notarize_args[@]}"
else
  echo "notarization skipped"
fi

if [[ -n "$update_url_base" ]]; then
  if [[ -z "${!update_key_env:-}" && "$dry_run" != "true" ]]; then
    echo "update metadata skipped: $update_key_env is not set"
  else
    [[ "$dry_run" == "true" || -n "$asset" ]] || fail "cannot make update metadata without a release asset"
    archive_name="${asset##*/}"
    update_url="${update_url_base%/}/$archive_name"
    update_args=("$cefari_command" make-update "$asset" --url "$update_url" --version "$version" --key-env "$update_key_env" --output-dir "$update_dir")
    [[ -n "$update_target" ]] && update_args+=(--target "$update_target")
    [[ -n "$update_format" ]] && update_args+=(--format "$update_format")
    run_cmd "${update_args[@]}"
  fi
else
  echo "update metadata skipped: update-url-base not provided"
fi

if [[ "$create_github_release" == "true" ]]; then
  if [[ -z "$release_tag" ]]; then
    release_tag="${GITHUB_REF_NAME:-}"
  fi
  if [[ -z "$release_tag" ]]; then
    echo "GitHub release skipped: release-tag not provided and GITHUB_REF_NAME is unavailable"
  elif [[ "$dry_run" == "true" ]]; then
    echo "+ gh release upload/create for $release_tag"
  elif command -v gh >/dev/null 2>&1; then
    release_args=(gh release create "$release_tag" "$asset" --title "${release_name:-$release_tag}")
    [[ "$mode" == "prerelease" ]] && release_args+=(--prerelease)
    if gh release view "$release_tag" >/dev/null 2>&1; then
      run_cmd gh release upload "$release_tag" "$asset" --clobber
    else
      run_cmd "${release_args[@]}"
    fi
  else
    fail "gh is required when create-github-release is true"
  fi
else
  echo "GitHub release creation skipped"
fi
