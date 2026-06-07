---
name: cefari
description: Use when working on Cefari app projects, templates, release workflows, packaging, daemon behavior, troubleshooting, or Cefari repo conventions; load the task-specific reference document before making changes.
---

# Cefari

Use this skill as a signpost. Read only the reference file that matches the current task:

- Project creation and `cefari.toml`: [project-creation.md](references/project-creation.md)
- Template authoring: [template-authoring.md](references/template-authoring.md)
- Release workflows and GitHub Actions: [release-workflows.md](references/release-workflows.md)
- Build and packaging behavior: [packaging.md](references/packaging.md)
- App daemon behavior: [daemon-behavior.md](references/daemon-behavior.md)
- Troubleshooting and verification: [troubleshooting.md](references/troubleshooting.md)

Keep Cefari runtime behavior in runtime crates and developer orchestration in `cefari-cli`. Prefer existing repository patterns over new abstractions.
