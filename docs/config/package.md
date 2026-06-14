# `[package]`

The `[package]` table controls app package metadata.

```toml
[package]
product_name = "My Cefari App"
version = "0.1.0"
```

## Fields

| Field | Required | Description |
| --- | --- | --- |
| `product_name` | Yes | Human-readable product name written into package metadata. |
| `version` | Yes | App version written into native package metadata. |

## Package Behavior

`cefari package` uses `product_name` and `version` when writing
`dist/package/` metadata for native packaging. Release automation can override
`version` with `cefari package --release-version VERSION`.

The app identifier still comes from `[app].identifier`, and executable names
still come from `[app].project_name`.
