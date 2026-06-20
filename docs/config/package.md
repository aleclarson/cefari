# `package`

The `package` section controls app package metadata.

```ts
package: {
  productName: "My Cefari App",
  version: "0.1.0",
}
```

## Fields

| Field | Required | Description |
| --- | --- | --- |
| `productName` | Yes | Human-readable product name written into package metadata. |
| `version` | Yes | App version written into native package metadata and runtime update checks. |

## Package Behavior

`cefari package` uses `productName` and `version` when writing `dist/package/`
metadata for native packaging. Release automation can override `version` with
`cefari package --release-version VERSION`.

The app identifier still comes from `app.identifier`, and executable names still
come from `app.projectName`.

Native resources are configured under top-level `nativeResources`, not in the
top-level `package` section. Workers and the daemon attach resources by listing
resource IDs in their own `native` lists. During package assembly, Cefari
includes only native resources selected for the build target recorded by
`cefari build`.
