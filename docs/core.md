# `cefari-core`

`cefari-core` is the reusable runtime library shared by the shipped desktop app. It intentionally avoids windowing, CEF initialization, CLI parsing, and packaging orchestration.

## Public Runtime Areas

### Paths

`AppIdentity` and `RuntimePaths` resolve platform-specific locations for:

- config
- data
- cache
- logs
- packaged resources
- update artifacts

### Config

`CefariConfig` models runtime app, update, and service configuration. `load_config` and `save_config` read and write JSON configuration with unknown-field rejection.

### Resources

`packaged_resources_dir` wraps `cargo-packager-resource-resolver`.

`resolve_resource` validates relative resource paths and reports missing resources explicitly.

### Logging

`RuntimeLogConfig` describes where desktop runtime logs should be written and what format should be requested.

`cefari-desktop` still owns installing the tracing subscriber because that must happen during native process startup and must keep appender guards alive.

### Updates

`UpdateCheckConfig` prepares `cargo-packager-updater` configuration.

`check_for_update` runs an update check and maps the outcome into `UpdateCheckState`.

`install_update` downloads and installs a returned updater package.

### Services

`CefariServiceSpec` builds service-manager contexts for daemon services.

The service helpers wrap install, uninstall, start, stop, restart, and status operations through `service-manager`.

## Non-Goals

`cefari-core` does not own:

- Tao windows
- CEF initialization
- native menus or tray icons
- CLI command parsing
- project scaffolding
- development, packaging, signing, notarization, or release orchestration

