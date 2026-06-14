# `app`

The `app` section defines the app's stable identity.

```ts
app: {
  projectName: "my-cefari-app",
  name: "My Cefari App",
  identifier: "dev.cefari.my-cefari-app",
  icon: "assets/icon.png",
  trayIcon: "assets/tray-icon.png",
}
```

## Fields

| Field | Required | Description |
| --- | --- | --- |
| `projectName` | Yes | Stable machine name for generated executables. |
| `name` | Yes | Human-readable app name used by developer-facing output and runtime shell chrome. |
| `identifier` | Yes | Reverse-DNS-style app identifier. |
| `icon` | No | Path to the app icon used for native package metadata. |
| `trayIcon` | Only when `capabilities.tray` is `true` | Path to the PNG icon used for OS tray/menu-bar integration. |

## `projectName`

`projectName` must be non-empty and contain only lowercase ASCII letters,
digits, and `-`.

Valid examples:

- `my-cefari-app`
- `demo1`

Invalid examples:

- `My-Cefari-App`
- `my_cefari_app`
- `my cefari app`
- `my.cefari.app`

Cefari uses `projectName` for generated executable names:

- desktop executable: `<projectName>` or `<projectName>.exe`
- daemon executable: `<projectName>-daemon` or `<projectName>-daemon.exe`

## Icons

`icon` is resolved relative to the project root and must point to a file when
provided. Use a square PNG, ideally `1024x1024`.

When `icon` is omitted, `cefari package` uses Cefari's default package icon.

`trayIcon` is resolved relative to the project root and must point to a PNG file
when provided. It is required only when `capabilities.tray` is `true`. Tray
icons are usually small, high-contrast, and template-style.
