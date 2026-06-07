# Troubleshooting

Use this reference when diagnosing Cefari build, dev, package, workflow, or
runtime failures in an app project. For product behavior and command details,
read:

- [docs/cli/index.md](../docs/cli/index.md)
- [docs/cli/diagnostics.md](../docs/cli/diagnostics.md)
- [docs/guides/build-and-package.md](../docs/guides/build-and-package.md)

## App Notes

- Start by confirming the failing command's working directory and whether the
  task is operating on the intended app project.
- Confirm `cefari.toml` exists and parses before investigating build or package
  outputs.
- Use `cefari doctor` and `cefari info` for local environment and project
  diagnostics.
