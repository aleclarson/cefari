# Project Decisions

These decisions capture the current implementation state. They should be revised when the corresponding implementation grows beyond the current scaffold.

## Platform Priority

Cefari currently verifies the workspace on macOS first. The desktop startup path has been run on macOS and creates:

- a runtime log file under `~/Library/Application Support/dev.Cefari.Cefari/logs/`
- a single-instance lock file under `~/Library/Caches/dev.Cefari.Cefari/`

Windows and Linux remain target platforms, but packaging, service operations, and desktop shell behavior still need platform-specific verification before they can be treated as supported.

## Frontend Template

`cefari init` currently generates a minimal static frontend:

- `frontend/index.html`
- configured dist path: `frontend/dist`

No JavaScript framework is selected yet. A richer template should be introduced only when `cefari dev` and `cefari build` define how frontend commands are run.

## Deno Daemon Shape

`cefari init` currently generates:

- `daemon/main.ts`
- configured daemon entry: `daemon/main.ts`

`cefari build` currently copies the daemon entry to `build/daemon/main.ts`.

The final packaged daemon output contract is not final. Packaging work should define whether the daemon remains TypeScript, is bundled, or is compiled before distribution.

## CEF Version

The workspace currently pins `cef = "148.4.0"`.

The dependency is optional in `cefari-desktop` until CEF initialization is implemented and verified in the desktop process.

## Package Identifiers And Signing

Generated projects use the app identifier from `cefari.toml` as the package identifier. `cefari init` currently derives that identifier from the display name, using the `dev.cefari.<slug>` shape.

Signing identities are not hard-coded into generated projects. They should be supplied by developer environment or CI configuration when `cefari codesign` and notarization workflows are implemented.

## Update Artifacts

Update artifact hosting is external to the app package. Runtime update checks consume configured endpoints and public keys; release automation is responsible for publishing compatible update metadata and signed artifacts.

`cefari make-update` should generate artifacts that match the `cargo-packager-updater` response shape documented by that crate.

## Generated Template Compatibility

Generated project templates are pre-release and may change before the first public distribution of `cefari-cli`.

Until compatibility is formalized, tests only guarantee that the current CLI can parse the current generated `cefari.toml` schema.
