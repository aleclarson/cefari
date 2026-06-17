# `capabilities`

The `capabilities` array opts an app into native desktop integrations.
Capabilities default to disabled when the array is omitted.

```ts
import { deepLinks, tray } from "cefari";

capabilities: [
  tray({
    icon: "assets/tray-icon.png",
  }),
  deepLinks({
    schemes: ["myapp"],
  }),
]
```

## Entries

| Entry | Required | Description |
| --- | --- | --- |
| `tray({ icon })` | No | Enables OS tray/menu-bar integration. |
| `deepLinks({ schemes })` | No | Registers URL schemes that the app can receive from the OS. |

## `tray({ icon })`

Add `tray({ icon })` when the app should appear in the OS tray or menu bar.

`icon` is required and must point to a PNG file relative to the project root.
`cefari dev` validates the icon before launching the desktop runtime, and
`cefari package` includes it as `tray-icon.png`.

Only one tray capability may be configured.

## `deepLinks({ schemes })`

Add `deepLinks({ schemes })` when the app should be registered as the native
handler for custom URL schemes in packaged builds.

`schemes` is required and must contain at least one URL scheme without `://`.
Each scheme must be lowercase ASCII, start with a letter, and contain only
letters, digits, `+`, `.`, or `-`.

The reserved schemes `http`, `https`, `file`, `mailto`, and `cefari` are not
allowed. A scheme may only appear once across all configured deep-link
capabilities.

`cefari package` writes the configured schemes into Cargo Packager's
`deep_link_protocols` metadata so packaged apps can be registered by the OS.
