# shellcheck shell=bash
# shellcheck disable=SC2154

infer_update_target() {
  echo "$update_target"
}

create_update_input_archive() {
  local target="$1"
  local archive="$update_input_dir/$target.zip"
  rm -rf "$update_input_dir"
  mkdir -p "$update_input_dir"
  rm -f "$archive"
  (
    cd "$package_dir/output" || return
    zip -qr "$archive" .
  )
  echo "$archive"
}
