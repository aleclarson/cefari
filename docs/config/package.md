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

Worker native payloads are configured under each worker's `native` list, not in
the top-level `package` section. During package assembly, Cefari includes only
native payloads selected for the build target recorded by `cefari build`.
