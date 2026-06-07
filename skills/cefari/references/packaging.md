# Packaging

Use this reference when changing `cefari build`, `cefari package`, native
packaging metadata, or release artifacts. For product behavior, read:

- [docs/guides/build-and-package.md](../docs/guides/build-and-package.md)
- [docs/cli/project.md](../docs/cli/project.md)
- [docs/config/package.md](../docs/config/package.md)
- [docs/architecture.md](../docs/architecture.md)

## Agent Notes

- Keep build/package orchestration in `cefari-cli`; keep reusable runtime helpers
  in `cefari-core`.
- When packaging behavior changes, update root `docs/` first and rerun
  `scripts/sync-cefari-skill-docs.rb`.
