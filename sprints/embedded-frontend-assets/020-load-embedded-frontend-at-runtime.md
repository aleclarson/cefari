# 020-load-embedded-frontend-at-runtime

## Task Title

Load embedded frontend at runtime

## Sprint Name

embedded-frontend-assets

## Sprint Objective

Bundle production frontend assets into the desktop executable while keeping the daemon as its own packaged executable.

## Base Branch Assumption

Detached `HEAD` at `48148d1`. The working tree was clean when this sprint was planned.

## Objective

Make `cefari-desktop` use embedded frontend assets when present, without changing daemon or CEF resource resolution.

## Scope

- Add a runtime module that materializes embedded frontend files to a deterministic resource directory under `RuntimePaths`, or serves them through the existing app-scheme path flow if that can be done with a similarly narrow diff.
- Prefer embedded frontend assets before packaged frontend resources when the embedded manifest is non-empty.
- Preserve `CEFARI_RESOURCE_DIR` override behavior for local diagnostics and explicit testing.
- Keep `RuntimeOperations::daemon_program()` resolving a real daemon executable from packaged/runtime resources.
- Add focused Rust tests for candidate ordering, missing embedded assets, fallback behavior, and path safety.

## Acceptance Criteria

- `cargo test -p cefari-desktop` passes.
- A desktop binary with an embedded frontend can resolve `frontend/index.html` without `frontend` being present in packaged resources.
- A desktop binary without an embedded frontend still uses the current packaged-resource and runtime-resource fallback behavior.
- The daemon path continues to point to `resource_dir/daemon/<daemon executable name>` or the package-resource equivalent, not embedded bytes.

## Review Checkpoint

Review runtime behavior: which resource source wins, where embedded frontend files are materialized if extraction is used, and whether diagnostics remain clear when frontend assets are unavailable.

## Work-Ahead Safety Note

One-task-ahead work is safe after this task because npm build wiring can target the established runtime contract. Packaging behavior should wait for this review if the resource location or precedence changes.

## Non-Obvious Prior-Task Assumptions

- Task `010-add-embedded-frontend-codegen` provides an embedded frontend manifest that is empty for generic builds and populated only when the desktop crate is rebuilt with a frontend directory.

## Risks, Ambiguities, Constraints, Or Review Checkpoints

- If materializing to disk, extraction should be idempotent and avoid rewriting unchanged assets on every launch.
- Keep CEF resources external because CEF expects native files and platform framework paths.
- Do not alter daemon service installation semantics in this task.
