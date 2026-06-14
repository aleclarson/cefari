# `[app]`

The `[app]` table defines the app's stable identity.

```toml
[app]
project_name = "my-cefari-app"
name = "My Cefari App"
identifier = "dev.cefari.my-cefari-app"
tray_icon = "assets/tray-icon.png"
icon = "assets/icon.png"
```

## Fields

| Field | Required | Description |
| --- | --- | --- |
| `project_name` | Yes | Stable machine name for generated executables. |
| `name` | Yes | Human-readable app name used by developer-facing output. |
| `identifier` | Yes | Reverse-DNS-style app identifier. |
| `tray_icon` | Yes | Path to the PNG icon used for OS tray/menu-bar integration. |
| `icon` | No | Path to the app icon used for native package metadata. |

## `project_name`

`project_name` must be non-empty and contain only lowercase ASCII letters,
digits, and `-`.

Valid examples:

- `my-cefari-app`
- `demo1`

Invalid examples:

- `My-Cefari-App`
- `my_cefari_app`
- `my cefari app`
- `my.cefari.app`

Cefari uses `project_name` for generated executable names:

- desktop executable: `<project_name>` or `<project_name>.exe`
- daemon executable: `<project_name>-daemon` or
  `<project_name>-daemon.exe`

## `icon`

`icon` is resolved relative to the project root and must point to a file.
Use a square PNG, ideally `1024x1024`.

When omitted, `cefari package` uses Cefari's default package icon.

## `tray_icon`

`tray_icon` is resolved relative to the project root and must point to a PNG
file. Cefari requires a tray icon because the desktop shell enables tray/menu-bar
integration.
