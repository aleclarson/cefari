# 050-document-and-verify-release-flow

## Task Title

Document and verify release flow

## Sprint Name

embedded-frontend-assets

## Sprint Objective

Bundle production frontend assets into the desktop executable while keeping the daemon as its own packaged executable.

## Base Branch Assumption

Detached `HEAD` at `48148d1`. The working tree was clean when this sprint was planned.

## Objective

Document the production asset layout and add final verification that proves frontend embedding works without changing daemon packaging.

## Scope

- Update `docs/guides/build-and-package.md` and other relevant docs to describe release embedded frontend behavior.
- Update `docs/spec.md` if the behavior is a supported product feature; read `.agents/rules/spec-doc.md` before editing `docs/spec.md`.
- Add or update verification scripts/tests if existing coverage does not inspect package payload shape.
- Run the Rust and npm test suites touched by the sprint.
- If feasible, run a smoke build/package fixture that confirms no external frontend resource is required for the desktop UI while the daemon remains separate.

## Acceptance Criteria

- Relevant docs describe which assets are embedded and which remain external.
- Feature inventory is updated in the correct docs location.
- Final verification includes Rust tests and npm tests changed by the sprint.
- A package/build inspection proves the daemon executable is still present as its own file.
- A package/build inspection proves frontend files are not duplicated as external package resources for embedded release builds.

## Review Checkpoint

Review the documented release contract and final verification summary. Confirm the sprint behavior is ready for human-run release packaging on each target platform.

## Work-Ahead Safety Note

No work-ahead is needed after this task; it is the sprint closeout.

## Non-Obvious Prior-Task Assumptions

- Task `040-package-embedded-frontend-releases` establishes the final package resource policy for embedded frontend release builds.

## Risks, Ambiguities, Constraints, Or Review Checkpoints

- Platform-specific packaging, signing, and notarization may require human-owned credentials or machines. Document human-run checks when the agent cannot run them locally.
- Do not claim CEF is embedded; CEF remains a packaged external resource in this sprint.
- Do not claim the daemon is embedded; it remains its own executable.
