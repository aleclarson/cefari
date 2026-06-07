---
name: cefari
description: Use when working on Cefari app projects, templates, release workflows, packaging, daemon behavior, troubleshooting, or Cefari repo conventions; use this skill as the entrypoint to task-specific references and copied docs.
---

# Cefari

Use this skill as a signpost. The reference files contain repo-specific agent
guidance. The copied docs contain the full product documentation.

## Task References

- Project creation, `cefari init`, generated structure, and template alignment:
  [project-creation.md](references/project-creation.md)
- Template changes under `templates/`:
  [template-authoring.md](references/template-authoring.md)
- Release workflows, GitHub Actions, signing, notarization, and updates:
  [release-workflows.md](references/release-workflows.md)
- Build, package, native metadata, and release artifacts:
  [packaging.md](references/packaging.md)
- Daemon execution, packaging, and service management:
  [daemon-behavior.md](references/daemon-behavior.md)
- Troubleshooting, verification, and focused test selection:
  [troubleshooting.md](references/troubleshooting.md)

## Docs Map

- Start here for app-user workflow: [getting-started.md](docs/getting-started.md)
- CLI commands: [cli/index.md](docs/cli/index.md),
  [cli/project.md](docs/cli/project.md),
  [cli/release.md](docs/cli/release.md),
  [cli/diagnostics.md](docs/cli/diagnostics.md)
- `cefari.toml`: [config/index.md](docs/config/index.md),
  [config/app.md](docs/config/app.md),
  [config/frontend.md](docs/config/frontend.md),
  [config/daemon.md](docs/config/daemon.md),
  [config/package.md](docs/config/package.md)
- Development, scaffolding, build, deployment, and native capabilities:
  [guides/development.md](docs/guides/development.md),
  [guides/scaffolding.md](docs/guides/scaffolding.md),
  [guides/build-and-package.md](docs/guides/build-and-package.md),
  [guides/deployment.md](docs/guides/deployment.md),
  [guides/native-capabilities.md](docs/guides/native-capabilities.md)
- TypeScript APIs and IPC:
  [typescript/index.md](docs/typescript/index.md),
  [typescript/namespaces.md](docs/typescript/namespaces.md),
  [typescript/raw-ipc.md](docs/typescript/raw-ipc.md),
  [typescript/events-and-errors.md](docs/typescript/events-and-errors.md),
  [ipc.md](docs/ipc.md)
- Architecture and runtime boundaries:
  [architecture.md](docs/architecture.md),
  [runtime/notifications.md](docs/runtime/notifications.md),
  [notifications.md](docs/notifications.md)
- Release action and workflow example:
  [release-action.md](docs/release-action.md),
  [examples/cefari-release-workflow.yml](docs/examples/cefari-release-workflow.yml)
- Verification and CSS contract:
  [verification.md](docs/verification.md),
  [css-contract.md](docs/css-contract.md)

The [docs](docs/) directory is generated from the repository root `docs/`
directory by `scripts/sync-cefari-skill-docs.rb`. Treat root `docs/` as the
source of truth. Use `rg` over this copied docs tree when a task needs
complete CLI, config, workflow, TypeScript, architecture, or runtime details.

Keep Cefari runtime behavior in runtime crates and developer orchestration in `cefari-cli`. Prefer existing repository patterns over new abstractions.
