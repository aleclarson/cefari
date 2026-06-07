# Packaging

Use this reference when changing `cefari build`, `cefari package`, native packaging metadata, or release artifacts.

## Responsibilities

- `cefari-cli` owns build, package, signing, notarization, diagnostics, and release orchestration.
- `cefari-desktop` is the internal shipped runtime crate, not the user-facing executable name.
- `cefari-core` owns reusable runtime helpers.

## White-Label Outputs

- Use `[app].project_name` for shipped executable names.
- Desktop output should be `<project_name>` or `<project_name>.exe`.
- Daemon output should be `<project_name>-daemon` or `<project_name>-daemon.exe`.
- Package metadata and manifests should refer to white-label output names.
- Preserve `cefari-cli` as the developer-facing tool name.

## Verification

- Confirm build outputs exist under `build/frontend`, `build/daemon`, and `build/desktop`.
- Confirm package metadata points at `build/desktop`.
- Confirm package manifests include white-label desktop and daemon executable names.
