# `[app]`

The `[app]` table defines the app's stable identity.

```toml
[app]
project_name = "my-cefari-app"
name = "My Cefari App"
identifier = "dev.cefari.my-cefari-app"
```

## Fields

| Field | Required | Description |
| --- | --- | --- |
| `project_name` | Yes | Stable machine name for generated executables. |
| `name` | Yes | Human-readable app name used by developer-facing output. |
| `identifier` | Yes | Reverse-DNS-style app identifier. |

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
