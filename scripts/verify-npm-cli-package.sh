#!/usr/bin/env bash
set -euo pipefail

if [[ "$#" -ne 2 ]]; then
  echo "Usage: verify-npm-cli-package.sh ROOT_PACKAGE_DIR PLATFORM_PACKAGE_DIR" >&2
  exit 2
fi

root_package_dir="$1"
platform_package_dir="$2"

test -f "$root_package_dir/package.json"
test -f "$platform_package_dir/package.json"

work_dir="$(mktemp -d)"
trap 'rm -rf "$work_dir"' EXIT

pack_dir="$work_dir/packs"
install_dir="$work_dir/install"
mkdir -p "$pack_dir" "$install_dir"

npm pack "$platform_package_dir" --pack-destination "$pack_dir" >/dev/null
npm pack "$root_package_dir" --pack-destination "$pack_dir" >/dev/null

platform_tarball="$(find "$pack_dir" -name 'cefari-cli-*.tgz' ! -name 'cefari-cli-[0-9]*.tgz' -print -quit)"
root_tarball="$(find "$pack_dir" -name 'cefari-cli-[0-9]*.tgz' -print -quit)"

test -f "$platform_tarball"
test -f "$root_tarball"

cd "$install_dir"
npm init -y >/dev/null
npm install "$platform_tarball" "$root_tarball" >/dev/null

cefari="./node_modules/.bin/cefari"
"$cefari" --version | grep -E '^cefari [0-9]+\.[0-9]+\.[0-9]+'
"$cefari" --help >/dev/null
"$cefari" init sample --name "NPM Package Sample" >/dev/null

mkdir -p cef-fixture
cat > cef-fixture/archive.json <<'JSON'
{
  "type": "minimal",
  "name": "cef_binary_148.0.10+gfixture+chromium-148.0.0_macosarm64_minimal.tar.bz2",
  "sha1": "fixture-sha1"
}
JSON
echo fixture > cef-fixture/libcef.fixture

env -u CEFARI_DESKTOP_RUNTIME CEFARI_CEF_RESOURCES_DIR="$install_dir/cef-fixture" "$cefari" build sample >/dev/null
test -f sample/build/desktop/npm-package-sample || test -f sample/build/desktop/npm-package-sample.exe
