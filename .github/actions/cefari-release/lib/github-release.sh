# shellcheck shell=bash
# shellcheck disable=SC2154

publish_github_release() {
  if [[ -z "$release_tag" ]]; then
    release_tag="${GITHUB_REF_NAME:-}"
  fi
  if [[ "$dry_run" == "true" ]]; then
    echo "+ gh release upload/create for $release_tag"
    return 0
  fi

  [[ -n "${GH_TOKEN:-${GITHUB_TOKEN:-}}" ]] || fail "GH_TOKEN or GITHUB_TOKEN is required when create-github-release is true"
  validate_command_available gh
  prepare_github_release_assets
  local create_release_args
  create_release_args=(gh release create "$release_tag" --title "${release_name:-$release_tag}")
  [[ "$mode" == "prerelease" ]] && create_release_args+=(--prerelease)
  if gh release view "$release_tag" >/dev/null 2>&1; then
    echo "GitHub release already exists: $release_tag"
  else
    run_cmd "${create_release_args[@]}"
  fi
  run_cmd gh release upload "$release_tag" "${github_release_assets[@]}" --clobber
}
