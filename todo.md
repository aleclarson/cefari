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

## 5. Verification

- [ ] Verify a freshly generated project uses white-label executable names end to end.
- [ ] Verify `templates/vite-react-basic/` remains runnable with the local Cefari build after the release workflow and skill-copy changes.
- [ ] Verify template production and prerelease workflows parse with `actionlint`.
- [ ] Verify the Cefari GitHub Action runs in a fixture workflow or dry-run mode.
- [ ] Move completed items from this file into `done.md` as soon as they are finished.
