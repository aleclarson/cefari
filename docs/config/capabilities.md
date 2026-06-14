# `[capabilities]`

The `[capabilities]` table opts an app into native desktop integrations.
Capabilities default to disabled when the table is omitted.

```toml
[capabilities]
tray = true
```

## Fields

| Field | Required | Description |
| --- | --- | --- |
| `tray` | No | Enables OS tray/menu-bar integration. Defaults to `false`. |

## `tray`

Set `tray = true` when the app should appear in the OS tray or menu bar.

When tray is enabled, `[app].tray_icon` is required and must point to a PNG file
relative to the project root. `cefari dev` validates that icon before launching
the desktop runtime, and `cefari package` includes it as `tray-icon.png`.
