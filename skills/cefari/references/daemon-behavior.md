# Daemon Behavior

Use this reference when changing Cefari daemon execution, service management,
or daemon packaging. For product behavior, read:

- [docs/config/daemon.md](../docs/config/daemon.md)
- [docs/guides/development.md](../docs/guides/development.md)
- [docs/guides/build-and-package.md](../docs/guides/build-and-package.md)
- [docs/guides/native-capabilities.md](../docs/guides/native-capabilities.md)

## Agent Notes

- Keep orchestration in `cefari-cli`; keep service installation and runtime
  behavior in runtime crates.
- Prefer tests around constructed service specs and package metadata before
  adding host-level service smoke checks.
