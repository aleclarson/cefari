# Troubleshooting

Use this reference when diagnosing Cefari build, dev, package, workflow, or
runtime failures. For product behavior and command details, read:

- [docs/cli/index.md](../docs/cli/index.md)
- [docs/cli/diagnostics.md](../docs/cli/diagnostics.md)
- [docs/verification.md](../docs/verification.md)
- [docs/guides/build-and-package.md](../docs/guides/build-and-package.md)

## Agent Notes

- Start by confirming the failing command's working directory and whether the
  task is operating on the repo or a generated project.
- Prefer focused crate or workflow checks before workspace-wide checks.
- When a test depends on local external tools, use fake tools or dry-run paths
  where the behavior under review does not require the real tool.
