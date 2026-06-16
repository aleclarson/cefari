---
name: cefari
description: Use when building Cefari app projects, configuring cefari.config.ts, using the Cefari CLI, packaging apps, adding native capabilities, release workflows, or troubleshooting app builds; use this skill as the entrypoint to app-developer references and copied docs.
---

# Cefari

Use this skill as a signpost for Cefari app development. The reference files
contain app-focused guidance. The copied docs contain app-developer product
documentation.

## Task References

- Project creation, template structure, and app setup:
  [project-creation.md](references/project-creation.md)
- Release workflows, GitHub Actions, signing, notarization, and updates:
  [release-workflows.md](references/release-workflows.md)
- Build, package, and release artifacts:
  [packaging.md](references/packaging.md)
- Daemon execution, packaging, and service management:
  [daemon-behavior.md](references/daemon-behavior.md)
- Troubleshooting app development, builds, packaging, and workflows:
  [troubleshooting.md](references/troubleshooting.md)

## Docs Map

- Start here for app-user workflow: [getting-started.md](docs/getting-started.md)
- CLI commands: [cli/index.md](docs/cli/index.md),
  [cli/project.md](docs/cli/project.md),
  [cli/release.md](docs/cli/release.md),
  [cli/diagnostics.md](docs/cli/diagnostics.md)
- `cefari.config.ts`: [config/index.md](docs/config/index.md),
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
- TypeScript APIs:
  [typescript/index.md](docs/typescript/index.md),
  [typescript/namespaces.md](docs/typescript/namespaces.md),
  [typescript/events-and-errors.md](docs/typescript/events-and-errors.md)
- Native behavior:
  [notifications.md](docs/notifications.md)
- Release action and workflow example:
  [release-action.md](docs/release-action.md),
  [examples/cefari-release-workflow.yml](docs/examples/cefari-release-workflow.yml)
- CSS contract: [css-contract.md](docs/css-contract.md)

Use `rg` over [docs](docs/) when a task needs complete CLI, config, workflow,
TypeScript, packaging, or native app capability details.
