# Project Creation

Use this reference when changing `cefari init`, `cefari.toml`, or generated
project structure. For product behavior, read:

- [docs/guides/scaffolding.md](../docs/guides/scaffolding.md)
- [docs/cli/project.md](../docs/cli/project.md)
- [docs/config/index.md](../docs/config/index.md)
- [docs/config/app.md](../docs/config/app.md)

## Agent Notes

- If generated files are intended to match templates, update both the init path
  and checked-in templates in the same change.
- Prefer parser tests for config contracts and integration tests for generated
  project contents.
