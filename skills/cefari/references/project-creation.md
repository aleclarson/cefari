# Project Creation

Use this reference when creating or adjusting a Cefari app project. For product
behavior, read:

- [docs/guides/scaffolding.md](../docs/guides/scaffolding.md)
- [docs/cli/project.md](../docs/cli/project.md)
- [docs/config/index.md](../docs/config/index.md)
- [docs/config/app.md](../docs/config/app.md)

## App Notes

- Prefer `cefari init` for new apps, then edit `cefari.config.ts` rather than
  recreating generated structure by hand.
- Use `app.name` for display text and `app.projectName` for stable
  machine-readable output names.
