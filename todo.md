# Cefari Todo

This file tracks current open work. Completed work belongs in [done.md](done.md).

Clarification note: the tasks below turn the requested outcomes into implementation-ready checklist items. Low-risk defaults are captured directly: project names are stable machine names, executable outputs should be white-labeled from that name, templates should run with the local Cefari build from a fresh checkout, and the Cefari skill should stay a compact signpost to task-oriented references.

## 1. White-Label Project Names And Executables

- [ ] Enforce a machine-readable project name in `cefari.toml`.
  - [ ] Add a required project-name field whose value must match `^[a-z0-9-]+$`.
  - [ ] Keep display/product names separate from the machine-readable project name.
  - [ ] Reject uppercase, spaces, underscores, punctuation outside `-`, empty names, and invalid TOML values with clear errors.
  - [ ] Update `cefari init` and checked-in templates to generate valid project names.
  - [ ] Add parser and integration tests for valid and invalid project names.
- [ ] Use the project name for white-label executable outputs.
  - [ ] Compile daemon outputs as `<project-name>-daemon` or `<project-name>-daemon.exe`.
  - [ ] Build or package desktop runtime outputs with project-specific executable names instead of `cefari-desktop`.
  - [ ] Ensure package metadata, package manifests, service specs, release payload verification, and docs refer to project-specific executable names.
  - [ ] Preserve Cefari CLI distribution as a separate developer tool, not part of app package white-labeling.
  - [ ] Verify native package payloads contain the white-label desktop and daemon executable names on macOS, Linux, and Windows.

## 2. Cefari Release GitHub Action

- [ ] Create a Cefari GitHub Action for common release tasks.
  - [ ] Define the action interface for project path, release/prerelease mode, target platforms, signing/notarization inputs, update metadata inputs, and artifact upload behavior.
  - [ ] Implement the action using the local Cefari CLI workflow: build, package, sign/notarize when configured, make update artifacts, and upload release assets.
  - [ ] Document required secrets and which steps are skipped when secrets are absent.
  - [ ] Add an example workflow that consumes the action from this repository.
  - [ ] Test the action with fixture or dry-run release inputs before treating it as supported.

## 3. Template Release Workflows

- [ ] Add built-in GitHub release workflows to every checked-in template.
  - [ ] Add a production release workflow for tagged stable releases.
  - [ ] Add a prerelease workflow for preview builds.
  - [ ] Make both workflows use the Cefari GitHub Action rather than duplicating release logic.
  - [ ] Ensure `templates/vite-react-basic/` can run the workflows with the local Cefari build or repo action path.
  - [ ] Document template-specific secrets, release triggers, and artifact expectations in the template README.
  - [ ] Add checks that template workflows stay in sync with the action interface.

## 4. Cefari Agent Skill

- [ ] Create and maintain `skills/cefari/SKILL.md` using the `skill-creator` guidance.
  - [ ] Keep `SKILL.md` concise, with YAML frontmatter containing only `name` and `description`.
  - [ ] Make `SKILL.md` a signpost to task-oriented reference documents.
  - [ ] Do not put setup or installation steps in `SKILL.md`.
  - [ ] Add one-level-deep reference documents for Cefari tasks, such as project creation, template authoring, release workflows, packaging, daemon behavior, and troubleshooting.
  - [ ] Avoid duplicating reference content in `SKILL.md`; move detailed workflows into `skills/cefari/references/`.
  - [ ] Validate the skill with the `skill-creator` validation workflow.
- [ ] Copy the Cefari skill into generated projects.
  - [ ] Update `cefari init` to copy `skills/cefari/` into `.agents/skills/cefari/`.
  - [ ] Ensure copied skills include `SKILL.md` and referenced task documents.
  - [ ] Add tests that generated projects contain `.agents/skills/cefari/SKILL.md`.
  - [ ] Ensure template projects also include or receive the Cefari skill copy path.

## 5. Advanced OS Notifications

- [ ] Add `user-notify` for advanced OS notification support.
  - [ ] Add `user-notify` to the appropriate runtime or desktop crate without pulling it into CLI-only code.
  - [ ] Define a small Cefari notification abstraction for user-visible desktop notifications.
  - [ ] Wire notification support into desktop runtime flows that need OS-level user feedback.
  - [ ] Document platform-specific permissions, fallbacks, and unsupported notification behavior.
  - [ ] Add tests or platform smoke checks for notification request construction and graceful failure paths.

## 6. CEF-To-Rust IPC Bridge

- [ ] Add a minimal typed CEF-to-Rust IPC bridge using `specta`.
  - [ ] Define one authoritative Rust command protocol for native app capabilities.
  - [ ] Derive `serde` and `specta::Type` for IPC commands, responses, events, and errors.
  - [ ] Export generated TypeScript definitions for templates and app frontends.
  - [ ] Add a small typed frontend wrapper around `window.cefari.invoke` and `window.cefari.on`.
  - [ ] Keep generated TypeScript types as the frontend contract instead of hand-written command strings.
- [ ] Route Rust-side and CEF-side native actions through one dispatcher.
  - [ ] Add a dispatcher that accepts typed IPC commands and operates on the native shell context.
  - [ ] Move menu, tray, window, update, service, logs, external-open, and notification-capable actions behind the dispatcher as they become available.
  - [ ] Ensure Rust-originated menu/tray actions call the same dispatcher as CEF-originated IPC requests.
  - [ ] Add checks or tests so no supported native shell action exists only as a Rust-only code path.
- [ ] Define the initial native command surface.
  - [ ] Include app quit, window show/focus/close/set-title, open logs, open validated external URL, update state/check, service status, and tray restore-window commands.
  - [ ] Model unsupported platform behavior as typed errors rather than silent no-ops.
  - [ ] Reserve notification commands for the `user-notify` integration without requiring notification support in the first IPC slice.
- [ ] Add IPC security and validation rules.
  - [ ] Inject the Cefari bridge only into trusted packaged app origins and explicitly allowed dev origins.
  - [ ] Validate all command arguments on the Rust side.
  - [ ] Avoid raw shell, arbitrary filesystem, process, or OS-handle access through IPC.
  - [ ] Keep external URL opening limited to approved schemes such as `http`, `https`, and `mailto`.
- [ ] Verify the IPC protocol.
  - [ ] Add serde round-trip tests for request, response, event, and error payloads.
  - [ ] Verify generated `specta` TypeScript definitions compile in the Vite React template.
  - [ ] Verify unknown, unsupported, denied, and invalid commands return typed errors.

## 7. Verification

- [ ] Verify a freshly generated project uses white-label executable names end to end.
- [ ] Verify `templates/vite-react-basic/` remains runnable with the local Cefari build after the release workflow and skill-copy changes.
- [ ] Verify template production and prerelease workflows parse with `actionlint`.
- [ ] Verify the Cefari GitHub Action runs in a fixture workflow or dry-run mode.
- [ ] Move completed items from this file into `done.md` as soon as they are finished.
