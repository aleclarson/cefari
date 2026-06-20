# `targets`

The `targets` section holds target-specific configuration for desktop, iOS, and
Android.

Desktop remains the only implemented runtime target. The mobile sections are
accepted so projects can declare future bundle identity and permissions without
mixing mobile-only fields into the generic app config.

```ts
targets: {
  desktop: {
    capabilities: [
      tray({ icon: "assets/tray-icon.png" }),
    ],
    daemon: {
      entry: "daemon/main.ts",
    },
  },
  ios: {
    bundleId: "dev.cefari.my-app",
    permissions: ["notifications"],
  },
  android: {
    applicationId: "dev.cefari.my_app",
    permissions: ["notifications"],
  },
}
```

## Desktop

`targets.desktop` can define desktop-only capabilities and daemon behavior.
When omitted, Cefari uses the top-level `capabilities` and `daemon` fields for
desktop builds.

| Field | Required | Description |
| --- | --- | --- |
| `capabilities` | No | Desktop native integrations such as tray and deep links. |
| `daemon` | No | Desktop Deno daemon entrypoint. |

## iOS

`targets.ios` records future iOS identity and permissions. It does not produce
an iOS app bundle yet.

| Field | Required | Description |
| --- | --- | --- |
| `bundleId` | No | iOS bundle identifier. Defaults to `app.identifier`. |
| `permissions` | No | Declared future iOS permissions. |

Desktop-only fields such as `daemon` and `capabilities` are rejected under
`targets.ios`.

## Android

`targets.android` records future Android identity and permissions. It does not
produce an Android project, APK, or app bundle yet.

| Field | Required | Description |
| --- | --- | --- |
| `applicationId` | No | Android application ID. Defaults to `app.identifier`. |
| `permissions` | No | Declared future Android permissions. |

Desktop-only fields such as `daemon` and `capabilities` are rejected under
`targets.android`.
