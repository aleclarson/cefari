# Project Decisions

These decisions capture the current implementation state. They should be revised when the corresponding implementation grows beyond the current scaffold.

## Platform Priority

Cefari currently verifies the workspace on macOS first. The desktop startup path has been run on macOS and creates:

- a runtime log file under `~/Library/Application Support/dev.Cefari.Cefari/logs/`
- a single-instance lock file under `~/Library/Caches/dev.Cefari.Cefari/`

Before the UI is available, desktop startup failures are reported to stderr with a stable `Cefari failed to start before the UI was available` prefix and, when tracing has already initialized, to the runtime log.

The desktop process now starts a Tao event loop and creates a blank `Cefari` main window. CEF initialization and packaged UI loading are still separate native shell tasks.

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

CEF preparation is owned by `cefari-cli`. The initial download source should be selected from the CEF distribution expected by the pinned Rust `cef` crate version. Automated download and cache behavior is still tracked separately in `todo.md`.

## Package Identifiers And Signing

Generated projects use the app identifier from `cefari.toml` as the package identifier. `cefari init` currently derives that identifier from the display name, using the `dev.cefari.<slug>` shape.

Signing identities are not hard-coded into generated projects. They are supplied by developer environment, CI configuration, or an explicit `sign.toml` path passed to `cefari codesign` and `cefari notarize`.

## Update Artifacts

Update artifact hosting is external to the app package. Runtime update checks consume configured endpoints and public keys; release automation is responsible for publishing compatible update metadata and signed artifacts.

`cefari make-update` signs a release archive through `cargo-codesign` and writes metadata that matches the `cargo-packager-updater` response shape documented by that crate.

## Generated Template Compatibility

Generated project templates are pre-release and may change before the first public distribution of `cefari-cli`.

Until compatibility is formalized, tests only guarantee that the current CLI can parse the current generated `cefari.toml` schema.
