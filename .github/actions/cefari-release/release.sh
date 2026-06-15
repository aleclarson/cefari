#!/usr/bin/env bash
set -euo pipefail

project_path="${CEFARI_PROJECT_PATH:-.}"
mode="${CEFARI_RELEASE_MODE:-release}"
targets="${CEFARI_TARGETS:-}"
cefari_command="${CEFARI_COMMAND:-cefari}"
install_cli="${CEFARI_INSTALL_CLI:-false}"
cefari_version="${CEFARI_CLI_VERSION:-}"
release_version="${CEFARI_RELEASE_VERSION:-}"
effective_version="$release_version"
release_tag="${CEFARI_RELEASE_TAG:-}"
release_name="${CEFARI_RELEASE_NAME:-}"
create_github_release="${CEFARI_CREATE_GITHUB_RELEASE:-false}"
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
release_artifacts_file="$project_path/dist/release-artifacts.txt"
github_release_assets_dir="$project_path/dist/github-release-assets"
update_input_dir="$project_path/dist/update-input"
release_assets=()
github_release_assets=()

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=.github/actions/cefari-release/lib/common.sh
source "$script_dir/lib/common.sh"
# shellcheck source=.github/actions/cefari-release/lib/assets.sh
source "$script_dir/lib/assets.sh"
# shellcheck source=.github/actions/cefari-release/lib/config-version.sh
source "$script_dir/lib/config-version.sh"
# shellcheck source=.github/actions/cefari-release/lib/update.sh
source "$script_dir/lib/update.sh"
# shellcheck source=.github/actions/cefari-release/lib/github-release.sh
source "$script_dir/lib/github-release.sh"

write_outputs() {
  {
    echo "package-dir=$package_dir"
    echo "update-dir=$update_dir"
    echo "artifact-dir=$artifact_dir"
    echo "release-artifacts=$release_artifacts_file"
    echo "release-mode=$mode"
  } >> "$GITHUB_OUTPUT"
}

[[ "$mode" == "release" || "$mode" == "prerelease" ]] || fail "mode must be release or prerelease"
bool_input "$create_github_release" || fail "create-github-release must be true or false"
bool_input "$install_cli" || fail "install-cli must be true or false"
bool_input "$notarize" || fail "notarize must be true or false"
bool_input "$dry_run" || fail "dry-run must be true or false"
[[ -n "$cefari_command" ]] || fail "cefari-command is required"
[[ "$install_cli" != "true" || -n "$cefari_version" ]] || fail "cefari-version is required when install-cli is true"
[[ -f "$project_path/cefari.config.ts" ]] || fail "cefari.config.ts not found at $project_path"
[[ -z "$update_url_base" || -n "$update_target" ]] || fail "update-target is required when update-url-base is set"
[[ -z "$signing_config" || -n "$signing_platform" ]] || fail "signing-platform is required when signing-config is set"
[[ "$notarize" != "true" || "$signing_platform" == "macos" ]] || fail "signing-platform must be macos when notarize is true"
[[ "$notarize" != "true" || -n "$signing_config" ]] || fail "signing-config is required when notarize is true"
if [[ "$create_github_release" == "true" && -z "$release_tag" && -z "${GITHUB_REF_NAME:-}" ]]; then
  fail "release-tag or GITHUB_REF_NAME is required when create-github-release is true"
fi
if [[ "$dry_run" == "true" && -z "$effective_version" ]]; then
  validate_command_available deno
  effective_version="$(read_project_package_version)"
  [[ -n "$effective_version" ]] || fail "release-version was not provided and package.version could not be read from cefari.config.ts"
fi

write_outputs

echo "Cefari release plan"
echo "  project: $project_path"
echo "  mode: $mode"
echo "  version: ${effective_version:-from package metadata}"
echo "  targets: ${targets:-current runner}"
echo "  cefari command: $cefari_command"
echo "  install cli: $install_cli"
echo "  dry-run: $dry_run"

if [[ "$install_cli" == "true" ]]; then
  validate_command_available npm
  run_cmd npm install -g "@cefari/cli@$cefari_version"
fi

validate_command_available "$cefari_command"

release_args=("$cefari_command" package release "$project_path" --mode "$mode")
[[ -n "$release_version" ]] && release_args+=(--version "$release_version")
[[ -n "$signing_platform" ]] && release_args+=(--signing-platform "$signing_platform")
[[ -n "$signing_config" ]] && release_args+=(--signing-config "$signing_config")
[[ "$notarize" == "true" ]] && release_args+=(--notarize)
[[ -n "$update_url_base" ]] && release_args+=(--update-url-base "$update_url_base")
[[ -n "$update_target" ]] && release_args+=(--update-target "$update_target")
[[ -n "$update_format" ]] && release_args+=(--update-format "$update_format")
[[ -n "$update_key_env" ]] && release_args+=(--update-key-env "$update_key_env")
[[ "$create_github_release" == "true" ]] && release_args+=(--github-release)
[[ -n "$release_tag" ]] && release_args+=(--release-tag "$release_tag")
[[ -n "$release_name" ]] && release_args+=(--release-name "$release_name")
[[ "$dry_run" == "true" ]] && release_args+=(--dry-run)
run_cmd "$cefari_command" build "$project_path" --release
run_cmd "${release_args[@]}"

if [[ "$dry_run" != "true" ]]; then
  effective_version="$(read_package_metadata_version)"
  [[ -n "$effective_version" ]] || fail "package metadata did not contain a version"
  collect_release_assets
else
  echo "artifact collection skipped in dry-run"
fi

if [[ -n "$signing_config" || -n "$signing_platform" ]]; then
  if [[ "$dry_run" == "true" ]]; then
    echo "signing skipped in dry-run: release artifacts are not collected"
  else
    effective_signing_platform="${signing_platform:-$(uname -s | tr '[:upper:]' '[:lower:]')}"
    case "$effective_signing_platform" in
      darwin) effective_signing_platform="macos" ;;
      mingw*|msys*|cygwin*) effective_signing_platform="windows" ;;
    esac
    for asset in "${release_assets[@]}"; do
      if is_signable_artifact "$asset" "$effective_signing_platform"; then
        sign_args=("$cefari_command" package sign "$asset")
        [[ -n "$signing_platform" ]] && sign_args+=(--platform "$signing_platform")
        [[ -n "$signing_config" ]] && sign_args+=(--config "$signing_config")
        run_cmd "${sign_args[@]}"
      else
        echo "signing skipped for unsupported artifact: $asset"
      fi
    done
  fi
else
  echo "signing skipped: no signing platform or signing config provided"
fi

if [[ "$notarize" == "true" ]]; then
  if [[ "$dry_run" == "true" ]]; then
    echo "notarization skipped in dry-run: release artifacts are not collected"
  else
    for asset in "${release_assets[@]}"; do
      if is_notarizable_artifact "$asset"; then
        notarize_args=("$cefari_command" package notarize "$asset")
        [[ -n "$signing_config" ]] && notarize_args+=(--config "$signing_config")
        run_cmd "${notarize_args[@]}"
      else
        echo "notarization skipped for unsupported artifact: $asset"
      fi
    done
  fi
else
  echo "notarization skipped"
fi

if [[ -n "$update_url_base" ]]; then
  if [[ -z "${!update_key_env:-}" && "$dry_run" != "true" ]]; then
    echo "update metadata skipped: $update_key_env is not set"
  else
    effective_update_target="$(infer_update_target)"
    [[ -n "$effective_update_target" ]] || fail "update-target is required when update-url-base is set"
    if [[ "$dry_run" == "true" ]]; then
      archive="$update_input_dir/$effective_update_target.zip"
    else
      validate_command_available zip
      archive="$(create_update_input_archive "$effective_update_target")"
    fi
    archive_name="${archive##*/}"
    update_url="${update_url_base%/}/$archive_name"
    update_args=("$cefari_command" package update "$archive" --url "$update_url" --version "$effective_version" --key-env "$update_key_env" --output-dir "$update_dir" --target "$effective_update_target")
    [[ -n "$update_format" ]] && update_args+=(--format "$update_format")
    run_cmd "${update_args[@]}"
  fi
else
  echo "update metadata skipped: update-url-base not provided"
fi

if [[ "$create_github_release" == "true" ]]; then
  publish_github_release
else
  echo "GitHub release creation skipped"
fi
