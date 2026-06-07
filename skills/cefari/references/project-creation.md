# Project Creation

Use this reference when changing `cefari init`, `cefari.toml`, or generated project structure.

## Config Contract

- `cefari.toml` is the project manifest.
- `[app].project_name` is the stable machine-readable app name.
- `project_name` must match `^[a-z0-9-]+$`.
- `[app].name` remains the human-facing display name.
- `[app].identifier` remains the app identifier.
- Do not derive user-facing copy from `project_name` when a display or product name exists.

## Init Behavior

- `cefari init` should create a runnable project from a fresh checkout.
- Generated projects should include frontend and daemon entry points.
- Defaults should be valid without manual edits.
- If generated files are intended to match templates, update both the init path and checked-in templates.

## Verification

- Add parser tests for config contracts.
- Add integration tests for generated project contents.
- Confirm invalid config fails with clear, actionable errors.
