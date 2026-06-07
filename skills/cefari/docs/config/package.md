# `[package]`

The `[package]` table controls app package metadata.

```toml
[package]
product_name = "My Cefari App"
```

## Fields

| Field | Required | Description |
| --- | --- | --- |
| `product_name` | Yes | Human-readable product name written into package metadata. |

## Package Behavior

`cefari package` uses `product_name` when writing `dist/package/` metadata for
native packaging.

The app identifier still comes from `[app].identifier`, and executable names
still come from `[app].project_name`.
