# Release Workflows

Use this reference when changing Cefari GitHub Actions, release workflow YAML, signing, notarization, or update artifact generation.

## Action Boundary

- The Cefari release action is the shared release entry point.
- The action should delegate build, package, signing, notarization, and update generation to `cefari-cli`.
- Keep workflow YAML thin; avoid duplicating release logic in templates.

## Expected Inputs

- Project path.
- Release or prerelease mode.
- Target platform list.
- Release version and tag.
- Signing and notarization config.
- Update metadata URL, target, format, and key env var.
- Artifact upload behavior.

## Secret Behavior

- Signing should skip when no signing inputs are provided.
- Notarization should run only when explicitly requested.
- Update metadata should require a public URL base and an update signing key.
- Missing explicitly required credentials should fail clearly.

## Verification

- Validate workflow YAML with `actionlint`.
- Use dry-run action inputs before relying on real credentials.
- Keep real signing and notarization checks separate from parse-only workflow validation.
