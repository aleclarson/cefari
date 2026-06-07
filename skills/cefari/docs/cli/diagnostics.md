# Diagnostics Commands

Use diagnostics commands to inspect local setup without changing project files.

## `cefari doctor`

Check whether common local tools are available:

```bash
cefari doctor
```

The command reports whether these tools can be executed:

- `cargo`
- `deno`
- `cargo-packager`
- `cargo-codesign`

`doctor` does not install missing tools.

## `cefari info`

Print CLI, host, and current-project information:

```bash
cefari info
```

The command reports:

- Cefari CLI version
- target operating system
- target architecture
- current project name and identifier when `cefari.toml` is present and valid

When the current directory is not a Cefari project, `info` reports that no
project was found. When `cefari.toml` exists but is invalid, it reports the
project as invalid with the parse error.
