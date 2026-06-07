# Troubleshooting

Use this reference when diagnosing Cefari build, dev, package, workflow, or
runtime failures.

## First Checks

- Confirm the command is being run from the expected repository or project
  directory.
- Confirm `cefari.toml` exists and parses.
- Confirm generated build artifacts exist before packaging.
- Confirm template commands match the repository root paths.

## Useful Commands

```bash
cefari --help
cefari info
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Common Failure Areas

- Missing `project_name` or invalid project names.
- Frontend dist path mismatch after a build command.
- Missing CEF resources.
- Host package tools running during tests that only need package assembly
  metadata.
- GitHub Actions workflows that parse but rely on unavailable signing secrets.

## Verification Notes

- Prefer focused tests for the changed crate before running workspace-wide
  checks.
- When a test depends on local external tools, use fake tools or dry-run paths
  where the behavior under review does not require the real tool.
