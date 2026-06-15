# shellcheck shell=bash
# shellcheck disable=SC2154

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
  local artifact
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

collect_release_assets() {
  local output_dir="$package_dir/output"
  local artifact
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
