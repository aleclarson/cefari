# 010-add-embedded-frontend-codegen

## Task Title

Add embedded frontend asset codegen

## Sprint Name

embedded-frontend-assets

## Sprint Objective

Bundle production frontend assets into the desktop executable while keeping the daemon as its own packaged executable.

## Base Branch Assumption

Detached `HEAD` at `48148d1`. The working tree was clean when this sprint was planned.

## Objective

Add a Rust build-time contract for embedding a frontend directory into `cefari-desktop` when production build tooling opts in.

## Scope

- Extend `crates/cefari-desktop/build.rs` to recognize an environment variable such as `CEFARI_EMBEDDED_FRONTEND_DIR`.
- Generate an `OUT_DIR` Rust module containing a deterministic manifest of frontend files and `include_bytes!` entries.
- Keep the generated embedded frontend absent when the env var is unset, so the generic desktop runtime remains generic.
- Preserve existing macOS Objective-C build behavior.
- Add focused Rust tests around manifest path validation, deterministic ordering, and generated metadata through testable helper functions where practical.

## Acceptance Criteria

- `cargo test -p cefari-desktop` passes.
- The build script rejects unsafe or non-file frontend entries such as parent-directory paths.
- The generated manifest stores relative frontend paths and byte references without including daemon or CEF assets.
- Generic builds without the env var compile and expose an empty/no-op embedded frontend manifest.

## Review Checkpoint

Review the build-time embedding contract: env var name, path validation, manifest shape, deterministic ordering, and proof that this task embeds only frontend assets.

## Work-Ahead Safety Note

One-task-ahead work is safe after this task because later runtime work can depend on the narrow embedded manifest contract without depending on npm packaging decisions.

## Non-Obvious Prior-Task Assumptions

None.

## Risks, Ambiguities, Constraints, Or Review Checkpoints

- Avoid adding a dependency unless path walking or hashing becomes materially complex.
- Keep generated code small and straightforward; compression is out of scope for this first slice.
- Do not embed the daemon. The daemon must remain a separate executable.
