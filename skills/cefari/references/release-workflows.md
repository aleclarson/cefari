# Release Workflows

Use this reference when changing Cefari GitHub Actions, release workflow YAML,
signing, notarization, or update artifact generation. For product behavior,
read:

- [docs/guides/deployment.md](../docs/guides/deployment.md)
- [docs/release-action.md](../docs/release-action.md)
- [docs/cli/release.md](../docs/cli/release.md)
- [docs/examples/cefari-release-workflow.yml](../docs/examples/cefari-release-workflow.yml)

## Agent Notes

- Keep workflow YAML thin; put release behavior in the shared action or
  `cefari-cli`.
- Use dry-run or parse-only validation before depending on real signing,
  notarization, or update credentials.
