# Release Workflows

Use this reference when setting up a Cefari app release workflow, signing,
notarization, or update artifacts. For product behavior, read:

- [docs/guides/deployment.md](../docs/guides/deployment.md)
- [docs/release-action.md](../docs/release-action.md)
- [docs/cli/release.md](../docs/cli/release.md)
- [docs/examples/cefari-release-workflow.yml](../docs/examples/cefari-release-workflow.yml)

## App Notes

- Start from the example workflow, then adjust `project-path`, release mode,
  signing inputs, notarization inputs, and update metadata for the app.
- Use dry-run or parse-only validation before relying on real release
  credentials.
