# shellcheck shell=bash
# shellcheck disable=SC2154

read_project_package_version() {
  local loader_dir
  loader_dir="$(mktemp -d)"
  trap 'rm -rf "$loader_dir"' RETURN
  cat > "$loader_dir/cefari-cli-config-api.js" <<'JS'
export function defineConfig(config) {
  return config;
}

export function tray(config) {
  return { type: "tray", ...config };
}
JS
  cat > "$loader_dir/import_map.json" <<'JSON'
{
  "imports": {
    "cefari": "./cefari-cli-config-api.js"
  }
}
JSON
  deno run \
    --quiet \
    "--allow-read=$project_path,$loader_dir" \
    --allow-env \
    --import-map "$loader_dir/import_map.json" \
    - "$project_path/cefari.config.ts" <<'JS'
import { pathToFileURL } from "node:url";

const config = (await import(pathToFileURL(Deno.args[0]).href)).default;
const version = config?.package?.version;
if (typeof version === "string") {
  console.log(version);
}
JS
}

read_package_metadata_version() {
  local metadata="$package_dir/cargo-packager.toml"
  [[ -f "$metadata" ]] || fail "package metadata not found at $metadata"
  awk '
    /^[[:space:]]*version[[:space:]]*=/ {
      line = $0
      sub(/^[^=]*=[[:space:]]*/, "", line)
      sub(/^[[:space:]]*"/, "", line)
      sub(/".*$/, "", line)
      print line
      exit
    }
  ' "$metadata"
}
