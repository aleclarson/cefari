# 030-wire-release-build-embedding

## Task Title

Wire release build embedding

## Sprint Name

embedded-frontend-assets

## Sprint Objective

Bundle production frontend assets into the desktop executable while keeping the daemon as its own packaged executable.

## Base Branch Assumption

Detached `HEAD` at `48148d1`. The working tree was clean when this sprint was planned.

## Objective

Update the TypeScript build flow so production release builds rebuild `cefari-desktop` with the freshly built frontend embedded.

## Scope

- Update `npm/src/build.ts` and nearby helpers so release builds pass the frontend output directory to the desktop Cargo build using the Rust embedding contract.
- Keep non-release builds and generic runtime copying behavior unchanged unless an explicit opt-in flag is added as part of the reviewed contract.
- Ensure `build/desktop/<projectName>` is the app-specific executable built with embedded frontend assets for release builds.
- Keep `build/daemon/<projectName>-daemon` as a separate executable built by `deno compile`.
- Add or update npm tests to verify release build command arguments, environment, output paths, and daemon separation.

## Acceptance Criteria

- `pnpm --dir npm test` passes, or the repo-equivalent npm test command passes if different.
- Release build tests prove the desktop Cargo build receives the frontend directory embedding env var.
- Build tests prove daemon compilation still writes a separate daemon executable under `build/daemon/`.
- Non-release build behavior remains compatible with current dev/source-checkout flows.

## Review Checkpoint

Review the CLI build contract: when embedding is enabled, whether it is tied to `--release`, and how app-specific desktop runtimes are distinguished from generic copied runtimes.

## Work-Ahead Safety Note

One-task-ahead work is safe after this task only for package metadata adjustments that depend on release builds producing an embedded frontend executable. Broader release behavior should wait for review if the trigger is changed from `--release`.

## Non-Obvious Prior-Task Assumptions

- Task `010-add-embedded-frontend-codegen` defines the Rust env var and generated manifest contract.
- Task `020-load-embedded-frontend-at-runtime` makes populated manifests usable at startup.

## Risks, Ambiguities, Constraints, Or Review Checkpoints

- The main ambiguity is whether embedding should be automatic for all `--release` builds or gated behind a new explicit build option first. This plan assumes automatic for release builds unless review changes it.
- Avoid new npm dependencies.
- Keep the daemon executable separate in build output and package input.
