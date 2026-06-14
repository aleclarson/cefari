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
release_artifacts_file="$project_path/dist/release-artifacts.txt"
github_release_assets_dir="$project_path/dist/github-release-assets"
update_input_dir="$project_path/dist/update-input"
release_assets=()
primary_release_asset=""
github_release_assets=()

write_outputs() {
  {
    echo "package-dir=$package_dir"
    echo "update-dir=$update_dir"
    echo "artifact-dir=$artifact_dir"
    echo "release-artifacts=$release_artifacts_file"
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

is_release_artifact() {
  local path="$1"
  case "$path" in
    *.app|*.dmg|*.app.tar.gz|*.AppImage|*.deb|*.rpm|*.exe|*.msi|*.zip|*.tar.gz) return 0 ;;
    *) return 1 ;;
  esac
}

is_signable_artifact() {
  local path="$1"
  local platform="$2"
  case "$platform:$path" in
    macos:*.app|macos:*.dmg) return 0 ;;
    linux:*.AppImage|linux:*.deb|linux:*.rpm|linux:*.tar.gz|linux:*.zip) return 0 ;;
    windows:*.exe|windows:*.msi|windows:*.zip) return 0 ;;
    *) return 1 ;;
  esac
}

is_notarizable_artifact() {
  local path="$1"
  case "$path" in
    *.app|*.dmg) return 0 ;;
    *) return 1 ;;
  esac
}

archive_directory_artifact() {
  local artifact="$1"
  local archive_name
  archive_name="$(basename "$artifact").tar.gz"
  mkdir -p "$github_release_assets_dir"
  run_cmd tar -czf "$github_release_assets_dir/$archive_name" -C "$(dirname "$artifact")" "$(basename "$artifact")"
  github_release_assets+=("$github_release_assets_dir/$archive_name")
}

prepare_github_release_assets() {
  github_release_assets=()
  rm -rf "$github_release_assets_dir"
  mkdir -p "$github_release_assets_dir"
  for artifact in "${release_assets[@]}"; do
    if [[ -f "$artifact" ]]; then
      github_release_assets+=("$artifact")
    elif [[ -d "$artifact" ]]; then
      archive_directory_artifact "$artifact"
    fi
  done
  if [[ -d "$update_dir" ]]; then
    while IFS= read -r artifact; do
      github_release_assets+=("$artifact")
    done < <(find "$update_dir" -type f -print | sort)
  fi
  [[ "${#github_release_assets[@]}" -gt 0 ]] || fail "no uploadable GitHub release assets were prepared"
}

infer_update_target() {
  if [[ -n "$update_target" ]]; then
    echo "$update_target"
    return 0
  fi
  if [[ -n "$targets" ]]; then
    echo "${targets%%,*}"
    return 0
  fi

  local os arch
  os="$(uname -s | tr '[:upper:]' '[:lower:]')"
  arch="$(uname -m | tr '[:upper:]' '[:lower:]')"
  case "$os" in
    darwin) os="macos" ;;
    mingw*|msys*|cygwin*) os="windows" ;;
  esac
  case "$arch" in
    arm64) arch="aarch64" ;;
    amd64) arch="x86_64" ;;
  esac
  echo "$os-$arch"
}

create_update_input_archive() {
  local target="$1"
  local archive="$update_input_dir/$target.zip"
  rm -rf "$update_input_dir"
  mkdir -p "$update_input_dir"
  rm -f "$archive"
  (
    cd "$package_dir/output"
    zip -qr "$archive" .
  )
  echo "$archive"
}

collect_release_assets() {
  local output_dir="$package_dir/output"
  [[ -d "$output_dir" ]] || fail "package output directory not found at $output_dir"

  release_assets=()
  while IFS= read -r artifact; do
    if is_release_artifact "$artifact"; then
      release_assets+=("$artifact")
    fi
  done < <(find "$output_dir" -mindepth 1 -maxdepth 1 \( -type f -o -type d \) -print | sort)

  [[ "${#release_assets[@]}" -gt 0 ]] || fail "no release artifacts found under $output_dir"
  mkdir -p "$(dirname "$release_artifacts_file")"
  : > "$release_artifacts_file"
  for artifact in "${release_assets[@]}"; do
    echo "$artifact" >> "$release_artifacts_file"
    echo "collected release artifact: $artifact"
  done
}

select_primary_release_asset() {
  primary_release_asset=""
  for artifact in "${release_assets[@]}"; do
    if [[ -f "$artifact" ]]; then
      primary_release_asset="$artifact"
      return 0
    fi
  done
  if [[ "${#release_assets[@]}" -gt 0 ]]; then
    primary_release_asset="${release_assets[0]}"
  fi
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
run_cmd "$cefari_command" package "$project_path" --release --release-version "$version"

if [[ "$dry_run" != "true" ]]; then
  collect_release_assets
  select_primary_release_asset
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
        sign_args=("$cefari_command" codesign "$asset")
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
        notarize_args=("$cefari_command" notarize "$asset")
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
    [[ -n "$effective_update_target" ]] || fail "update target could not be inferred"
    if [[ "$dry_run" == "true" ]]; then
      archive="$update_input_dir/$effective_update_target.zip"
    else
      validate_command_available zip
      archive="$(create_update_input_archive "$effective_update_target")"
    fi
    archive_name="${archive##*/}"
    update_url="${update_url_base%/}/$archive_name"
    update_args=("$cefari_command" make-update "$archive" --url "$update_url" --version "$version" --key-env "$update_key_env" --output-dir "$update_dir" --target "$effective_update_target")
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
  else
    [[ -n "${GH_TOKEN:-${GITHUB_TOKEN:-}}" ]] || fail "GH_TOKEN or GITHUB_TOKEN is required when create-github-release is true"
    validate_command_available gh
    prepare_github_release_assets
    create_release_args=(gh release create "$release_tag" --title "${release_name:-$release_tag}")
    [[ "$mode" == "prerelease" ]] && create_release_args+=(--prerelease)
    if gh release view "$release_tag" >/dev/null 2>&1; then
      echo "GitHub release already exists: $release_tag"
    else
      run_cmd "${create_release_args[@]}"
    fi
    run_cmd gh release upload "$release_tag" "${github_release_assets[@]}" --clobber
  fi
else
  echo "GitHub release creation skipped"
fi
