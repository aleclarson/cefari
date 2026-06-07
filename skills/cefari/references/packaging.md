# Packaging

Use this reference when building, packaging, or inspecting Cefari app release
artifacts. For product behavior, read:

- [docs/guides/build-and-package.md](../docs/guides/build-and-package.md)
- [docs/cli/project.md](../docs/cli/project.md)
- [docs/config/package.md](../docs/config/package.md)

## App Notes

- Run `cefari build` before `cefari package` when package inputs are missing or
  stale.
- Check `build/frontend`, `build/daemon`, `build/desktop`, and `dist/package`
  when diagnosing missing artifacts.
