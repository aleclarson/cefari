#!/usr/bin/env bash
set -euo pipefail

if [[ "$#" -ne 2 ]]; then
  echo "usage: $0 OUTPUT_DIR INSPECT_DIR" >&2
  exit 2
fi

output_dir="$1"
inspect_dir="$2"
rm -rf "$inspect_dir"
mkdir -p "$inspect_dir"

case "${RUNNER_OS:-$(uname -s)}" in
  macOS | Darwin)
    app="$(find "$output_dir" -maxdepth 1 -name '*.app' -type d | head -n 1)"
    test -n "$app"
    cp -R "$app" "$inspect_dir/app"
    find "$output_dir" -maxdepth 1 -name '*.dmg' -type f -print -quit | grep -q .
    ;;
  Linux)
    deb="$(find "$output_dir" -maxdepth 1 -name '*.deb' -type f | head -n 1 || true)"
    appimage="$(find "$output_dir" -maxdepth 1 -name '*.AppImage' -type f | head -n 1 || true)"
    if [[ -n "$deb" ]]; then
      dpkg-deb -x "$deb" "$inspect_dir"
    elif [[ -n "$appimage" ]]; then
      chmod +x "$appimage"
      (cd "$inspect_dir" && "$GITHUB_WORKSPACE/$appimage" --appimage-extract)
    else
      cp -R "$output_dir"/. "$inspect_dir"
    fi
    ;;
  Windows)
    artifact="$(find "$output_dir" -maxdepth 1 \( -name '*.exe' -o -name '*.msi' \) -type f | head -n 1 || true)"
    test -n "$artifact"
    7z x -y -o"$inspect_dir" "$artifact" >/dev/null
    ;;
  *)
    cp -R "$output_dir"/. "$inspect_dir"
    ;;
esac
