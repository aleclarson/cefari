# Daemon Behavior

Use this reference when changing Cefari daemon execution, service management, or daemon packaging.

## Development

- `cefari dev` may run the daemon directly from the project daemon entry point.
- Dev orchestration should keep frontend, daemon, and desktop processes tied together.
- Dev server port behavior should follow `frontend.dev_port` unless explicitly overridden.

## Build And Package

- Build output should include the daemon source copy and compiled daemon executable.
- The compiled daemon executable should use the project white-label name.
- Package manifests should identify the daemon executable path.

## Runtime Service Management

- Service install/start/stop/status belongs to runtime code.
- Keep service labels and executable paths explicit and testable.
- Treat platform-specific service behavior as a first-class compatibility concern.

## Verification

- Test daemon executable naming on host platform conventions.
- Test service spec construction separately from real service installation when possible.
- Use platform smoke checks for real service operations.
