# `capabilities`

The `capabilities` section opts an app into native desktop integrations.
Capabilities default to disabled when the section is omitted.

```ts
capabilities: {
  tray: true,
}
```

## Fields

| Field | Required | Description |
| --- | --- | --- |
| `tray` | No | Enables OS tray/menu-bar integration. Defaults to `false`. |

## `tray`

Set `tray` to `true` when the app should appear in the OS tray or menu bar.

When tray is enabled, `app.trayIcon` is required and must point to a PNG file
relative to the project root. `cefari dev` validates that icon before launching
the desktop runtime, and `cefari package` includes it as `tray-icon.png`.
