# Proposal: Mobile Platform Readiness

Cefari should shape its internals now so iOS and Android can become first-class
runtime targets without requiring a rewrite of the desktop architecture. Mobile
support is not implemented by this proposal. The goal is to make current desktop
work land in places that can be shared, replaced, or explicitly scoped when
mobile hosts arrive.

## Goals

- Keep product-facing app APIs portable by default.
- Keep platform-specific behavior behind narrow runtime host boundaries.
- Preserve the Rust-owned, typed IPC contract as the shared app capability
  surface.
- Let desktop, iOS, and Android runtimes use different webview, lifecycle,
  packaging, and permission systems.
- Make unsupported capability behavior explicit at config, type, runtime, and
  documentation boundaries.
- Avoid baking CEF, Tao, desktop windowing, or desktop packaging assumptions
  into `cefari-core` or frontend APIs.

## Non-Goals

- Do not implement iOS or Android runtime hosts in the first pass.
- Do not promise that every desktop capability will exist on mobile.
- Do not introduce compatibility shims for existing pre-alpha internals.
- Do not design a cross-platform UI toolkit or native widget abstraction.
- Do not require mobile targets to use CEF or desktop process structure.
- Do not model app-store submission, code signing, or push notification provider
  details before the runtime boundaries are clear.

## Target Shape

Cefari should keep one product API with multiple native hosts:

```text
frontend app
  -> cefari/app TypeScript API
  -> typed Cefari IPC envelope
  -> platform runtime host
       -> desktop host: CEF, Tao, menus, tray, updater, daemon
       -> iOS host: WKWebView, UIKit/Swift host, iOS entitlements
       -> Android host: WebView, Activity/service host, Android permissions
```

The high-level crate and package split should become:

```text
crates/
  cefari-core/        shared config, paths, typed IPC, resources, capability metadata
  cefari-desktop/     desktop host implementation
  cefari-ios/         future iOS host implementation
  cefari-android/     future Android host implementation

npm/
  src/app/            portable frontend API wrappers
  src/cli/            developer commands, target selection, build orchestration
  src/platform/       host-specific packaging and dev adapters
```

The exact module names can change, but the ownership rule should not:
`cefari-core` defines cross-platform contracts, and each host crate owns native
runtime behavior for one platform family.

## Platform Host Boundary

The desktop runtime currently owns CEF startup, Tao windows, native menus, tray
integration, notifications, runtime logging, and native action dispatch. Mobile
support needs the same ownership pattern, but with host-specific implementations.

Each runtime host should own:

- Webview creation and bridge injection.
- Native event-loop and lifecycle integration.
- Capability dispatch for commands supported by that host.
- Permission prompts and native entitlement checks.
- Runtime logging sinks and crash-reporting hooks.
- App resource loading for development and packaged builds.
- Host-specific process, activity, scene, or window lifecycle.

Each runtime host should not own:

- The shared IPC wire envelope.
- Frontend TypeScript API names and result shapes.
- Cross-platform config schema definitions.
- Capability metadata shared across hosts.
- Developer CLI orchestration that is not shipped in the app.

The core architectural test is whether a capability can be reasoned about
without knowing whether the current host is desktop, iOS, or Android. If it can,
the contract belongs in `cefari-core`. If it needs AppKit, UIKit, Android SDK,
Tao, CEF, or desktop package state, the behavior belongs in the host crate.

## Capability Model

Every native capability should declare its platform support status. This
metadata should live with the capability contract rather than in a distant
registry.

Recommended support classes:

- `portable`: same semantic API across desktop, iOS, and Android.
- `hostSpecific`: same API name, but host-specific behavior or limits.
- `desktopOnly`: desktop concept with no mobile equivalent.
- `mobileOnly`: mobile concept with no desktop equivalent.
- `deferred`: intentionally reserved but not implemented.

Examples:

| Capability | Likely status | Notes |
| --- | --- | --- |
| App metadata and lifecycle | `hostSpecific` | Mobile foreground/background lifecycle is not desktop window lifecycle. |
| Windows | `desktopOnly` initially | Mobile should expose screens, routes, or presentation primitives later instead of fake desktop windows. |
| Dialogs | `hostSpecific` | Native sheets, activities, and permission flows differ by host. |
| Files | `hostSpecific` | Mobile storage is sandboxed and permission-mediated. |
| Notifications | `hostSpecific` | Local notification APIs may align; remote push is a separate capability. |
| Shell/open URL | `hostSpecific` | External URL handling, app links, and intents differ. |
| Tray and menus | `desktopOnly` | These should not leak into portable app code as required concepts. |
| Downloads | `hostSpecific` | Mobile download visibility and storage location require explicit policy. |
| Updates | `desktopOnly` initially | App-store updates should not share the desktop updater contract. |
| Workers/daemon | `hostSpecific` | Long-running background work has strict mobile lifecycle limits. |

Unsupported behavior should fail predictably:

- TypeScript wrappers expose support checks or host-specific namespaces for
  non-portable capabilities.
- IPC dispatch returns typed unsupported-capability errors.
- Config validation rejects impossible target/capability combinations before
  packaging.
- Documentation lists platform support for each public capability.

## Frontend API Design

The `cefari/app` package should stay portable by default. Desktop-specific APIs
can exist, but they should be semantically and structurally separate from the
common surface.

Recommended shape:

```ts
import { cefari } from "cefari/app";

await cefari.app.info();
await cefari.notifications.requestPermission();

if (cefari.platform.isDesktop) {
  await cefari.desktop.tray.setMenu(...);
}
```

Guidelines:

- Keep common APIs under stable namespaces such as `app`, `files`,
  `notifications`, `shell`, and `dialogs` only when their semantics can be
  stated across hosts.
- Move desktop-only APIs under explicit desktop namespaces before they become
  widely copied by apps.
- Avoid frontend code that infers platform behavior from browser user agents.
- Generate TypeScript platform-support metadata from the same Rust capability
  metadata used by runtime dispatch.
- Treat mobile permissions as first-class API results, not incidental native
  errors.

## Configuration And Manifests

`cefari.config.ts` should grow toward target-aware configuration rather than a
single desktop-shaped manifest.

A future shape could be:

```ts
export default defineCefariConfig({
  app: {
    id: "com.example.app",
    name: "Example",
  },
  targets: {
    desktop: {
      windows: { main: { width: 1200, height: 800 } },
    },
    ios: {
      bundleId: "com.example.app",
      permissions: ["notifications"],
    },
    android: {
      applicationId: "com.example.app",
      permissions: ["notifications"],
    },
  },
});
```

Preparation work:

- Separate app identity from desktop package identity.
- Keep target-specific fields under target-specific keys.
- Validate that portable config can be consumed without loading desktop
  packaging code.
- Model permissions declaratively so builds can produce native entitlements,
  manifests, and documentation.
- Keep dev-server, resource, daemon, and package paths target-aware.

## Build And Development Tooling

The npm package should treat platform targets as orchestration choices, not as
runtime logic.

Needed structure:

- A target model such as `desktop`, `ios`, and `android`.
- Shared frontend build steps that can be reused by every target.
- Host-specific dev adapters for launching the native shell.
- Host-specific package adapters for app bundles, archives, installers, or
  store-ready artifacts.
- Target-aware diagnostics that report missing SDKs, simulators, signing
  material, or platform permissions.

The CLI should avoid assuming that `cefari dev`, `cefari build`, or
`cefari package` always means desktop. Desktop can remain the default while
targets are added:

```text
cefari dev --target desktop
cefari dev --target ios
cefari package --target android
```

## Runtime Lifecycle

Desktop lifecycle is centered on process, windows, tray, menus, and updater
state. Mobile lifecycle is centered on foreground/background state, scenes or
activities, permissions, OS termination, and app-store update rules.

Preparation work:

- Define a portable app lifecycle event vocabulary before adding mobile events.
- Keep window lifecycle events separate from app lifecycle events.
- Avoid using the main desktop window as the universal app lifecycle anchor.
- Model background work as an explicit capability with platform limits.
- Persist runtime state through host-owned lifecycle hooks rather than desktop
  shutdown paths.

## Daemon And Background Work

Cefari's daemon and worker concepts need a mobile review before they become
portable promises.

Desktop can run a local daemon as part of the app development and packaged
runtime story. iOS and Android have stricter constraints on long-running work,
networking, background execution, and process ownership.

Preparation work:

- Treat desktop daemon support as a host-specific capability.
- Keep frontend worker APIs separate from native background services.
- Define which background operations are portable, permission-gated, or
  unsupported.
- Do not assume one always-on companion process exists on mobile.

## Resources And Storage

Resource loading and file access should be expressed through logical Cefari
paths and capability APIs instead of native filesystem paths leaking into
frontend code.

Preparation work:

- Keep packaged frontend assets behind host-owned resource loaders.
- Keep app data, cache, logs, downloads, and temporary files as logical
  locations in `cefari-core`.
- Require host adapters to map logical locations to platform directories.
- Make user-visible files permission-mediated on mobile.
- Avoid desktop path separators, absolute paths, and executable path assumptions
  in shared contracts.

## Testing Strategy

Mobile readiness needs contract tests before mobile hosts exist.

- Add capability metadata tests that ensure every IPC capability declares
  platform support.
- Add generated TypeScript tests for unsupported-capability result handling.
- Add config validation tests for target-specific fields.
- Add resource path tests that do not assume desktop filesystem layout.
- Add host adapter tests around lifecycle, permission, and resource loading as
  each mobile runtime lands.
- Keep desktop smoke tests focused on desktop behavior so they do not become
  accidental cross-platform specifications.

## Documentation Work

Docs should distinguish current desktop support from future mobile-readiness
work.

- Keep supported behavior in `docs/spec.md`.
- Keep forward-looking platform design in proposal docs until implemented.
- Add platform-support tables to public capability docs when metadata exists.
- Document unsupported mobile behavior as unsupported, not as a desktop caveat.
- Avoid examples that rely on tray, menus, windows, local daemons, or desktop
  updater APIs unless the guide is explicitly desktop-specific.

## First Steps

1. Add platform-support metadata beside each IPC capability contract.
2. Split frontend APIs into common and desktop-explicit namespaces where needed.
3. Make config parsing target-aware without adding mobile build output yet.
4. Move desktop packaging assumptions out of shared build helpers.
5. Define a portable app lifecycle event vocabulary.
6. Audit logical paths, resources, permissions, and daemon usage for desktop
   assumptions.
7. Add tests that prevent new shared contracts from depending on desktop-only
   concepts.

## Open Questions

- Should mobile hosts be Rust-first wrappers around Swift/Kotlin code, or should
  Swift and Kotlin own the host with Rust limited to shared protocol code?
- Should mobile apps support the same daemon API through constrained background
  tasks, or should daemon stay desktop-only?
- Should `cefari.window` remain a desktop namespace once mobile support starts,
  or should it be renamed before public adoption grows?
- Should capability metadata be hand-written Rust data, generated from module
  conventions, or declared in a separate manifest format?
- Should `cefari dev --target ios` launch simulators directly, or only prepare
  an Xcode project/workspace at first?
