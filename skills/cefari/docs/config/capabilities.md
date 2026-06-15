# `capabilities`

The `capabilities` array opts an app into native desktop integrations.
Capabilities default to disabled when the array is omitted.

```ts
import { tray } from "@cefari/cli";

capabilities: [
  tray({
    icon: "assets/tray-icon.png",
  }),
]
```

## Entries

| Entry | Required | Description |
| --- | --- | --- |
| `tray({ icon })` | No | Enables OS tray/menu-bar integration. |

## `tray({ icon })`

Add `tray({ icon })` when the app should appear in the OS tray or menu bar.

`icon` is required and must point to a PNG file relative to the project root.
`cefari dev` validates the icon before launching the desktop runtime, and
`cefari package` includes it as `tray-icon.png`.

Only one tray capability may be configured.
