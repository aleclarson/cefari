# 040-package-embedded-frontend-releases

## Task Title

Package embedded frontend releases

## Sprint Name

embedded-frontend-assets

## Sprint Objective

Bundle production frontend assets into the desktop executable while keeping the daemon as its own packaged executable.

## Base Branch Assumption

Detached `HEAD` at `48148d1`. The working tree was clean when this sprint was planned.

## Objective

Adjust package assembly so release packages no longer include `frontend` as an external resource when the desktop executable already embeds it, while continuing to package the daemon and CEF resources.

## Scope

- Update `npm/src/package.ts` package metadata generation to omit the `frontend` resource for embedded-frontend release builds.
- Preserve external resource entries for `daemon` and `cef`.
- Preserve tray/icon handling according to the reviewed runtime decision. If tray icon remains file-based, keep it packaged externally.
- Add package manifest metadata that makes embedded frontend status explicit for diagnostics and release inspection.
- Add or update npm package tests for resource entries and manifest fields.

## Acceptance Criteria

- `pnpm --dir npm test` passes, or the repo-equivalent npm test command passes if different.
- Package metadata for embedded release builds does not list `frontend` as a `[[resources]]` target.
- Package metadata still lists the daemon resource and validates the daemon executable exists.
- Package metadata still lists the CEF resource directory and validates `archive.json` exists.
- Package manifest clearly records that the frontend is embedded.

## Review Checkpoint

Review native package assembly output: confirm the package contains a separate daemon executable and CEF resources, but not duplicated frontend files.

## Work-Ahead Safety Note

One-task-ahead work is safe after this task because docs and final verification can describe the accepted package behavior. Implementation work beyond docs should wait if resource inclusion policy changes.

## Non-Obvious Prior-Task Assumptions

- Task `030-wire-release-build-embedding` ensures release builds produce a desktop executable with embedded frontend assets before package assembly runs.

## Risks, Ambiguities, Constraints, Or Review Checkpoints

- Existing `cefari package` can be run after a non-release build. The task must decide how to detect embedded frontend status from build artifacts or package options without guessing incorrectly.
- Do not remove daemon validation or package daemon bytes into the desktop executable.
- CEF remains external in this sprint.
