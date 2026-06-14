# Proposal: `cefari.config.ts`

## Summary

Replace `cefari.toml` with a TypeScript project configuration file named
`cefari.config.ts`.

The new configuration API should use the familiar app-tooling shape:

```ts
import { defineConfig } from "@cefari/cli";

export default defineConfig({
  app: {
    projectName: "my-cefari-app",
    name: "My Cefari App",
    identifier: "dev.cefari.my-cefari-app",
    icon: "assets/icon.png",
    trayIcon: "assets/tray-icon.png",
  },
  frontend: {
    dist: "frontend/dist",
    devPort: 5173,
    buildCommand: ["deno", "task", "build"],
    devCommand: ["deno", "task", "dev", "--", "--port", "{port}"],
  },
  daemon: {
    entry: "daemon/main.ts",
  },
  package: {
    productName: "My Cefari App",
    version: "0.1.0",
  },
});
```

This should be a breaking replacement, not a compatibility layer. Cefari is
pre-alpha, and keeping both TOML and TypeScript loaders would add legacy surface
area before the project has earned it.

## Goals

- Make Cefari project configuration feel native to TypeScript app authors.
- Provide typed editor feedback for app identity, frontend, daemon, packaging,
  icon, and future release settings.
- Keep the CLI as the canonical runtime validator and executor for config-driven
  behavior.
- Run the TypeScript config with Deno.
- Export `defineConfig` and config types from the `@cefari/cli` package.
- Keep the runtime boundary intact: app project config remains CLI input, not
  desktop runtime state.
- Fail loudly on unknown fields, invalid shapes, missing required values, and
  non-serializable config exports.

## Non-Goals

- Do not keep `cefari.toml` as a supported project manifest.
- Do not add migration shims, TOML fallback loading, or dual-format precedence.
- Do not make the Rust CLI depend on Node-only TypeScript execution.
- Do not move desktop runtime configuration into the project config file.
- Do not support arbitrary runtime objects such as functions, classes, symbols,
  streams, or file handles in the exported config value.

## User-Facing Contract

`cefari.config.ts` lives in the project root. The CLI loads it for commands that
currently load `cefari.toml`: `dev`, `build`, `package`, `clean`, and `info`.

The config file is ordinary TypeScript executed by Deno. It may import local
files, read environment variables, branch on platform, and compute values.
After execution, the default export must be JSON-serializable project config.

The `defineConfig` helper should be intentionally small:

```ts
export function defineConfig(config: CefariConfigInput): CefariConfigInput {
  return config;
}
```

Its value is type inference and editor support, not hidden runtime behavior.
The helper must not be treated as validation. The CLI remains responsible for
runtime validation and defaults.

## Import Surface

`@cefari/cli` should export:

- `defineConfig`
- `CefariConfigInput`
- section input types such as `AppConfigInput`, `FrontendConfigInput`,
  `DaemonConfigInput`, and `PackageConfigInput`

The package also remains the npm CLI distribution with the `cefari` binary.
The npm package should add an `exports` map for the config API while preserving
the existing `bin` entry.

Recommended package shape:

```json
{
  "name": "@cefari/cli",
  "type": "module",
  "bin": {
    "cefari": "bin/cefari.js"
  },
  "exports": {
    ".": {
      "types": "./config/index.d.ts",
      "default": "./config/index.js"
    }
  },
  "files": [
    "bin/cefari.js",
    "config/index.js",
    "config/index.d.ts",
    "README.md"
  ]
}
```

Because the current bin wrapper is CommonJS, the implementation can either keep
that file as `.cjs` or update it to ESM as part of the npm package change.

## Deno Loader Design

The Rust CLI should not parse TypeScript. It should delegate config execution to
Deno and consume normalized JSON.

Recommended flow:

1. Resolve `<project>/cefari.config.ts`.
2. Spawn Deno with a small loader script owned by Cefari.
3. The loader imports the config file by file URL.
4. The loader reads the default export.
5. The loader rejects missing exports and non-object exports.
6. The loader verifies the value can be serialized with `JSON.stringify`.
7. The loader prints only JSON to stdout.
8. Rust deserializes the JSON into `ProjectConfig` with `serde`.
9. Rust performs the same strict validation used by command execution.

The Deno loader should receive the config path as an argument, not interpolate it
into evaluated source. That keeps paths with spaces and unusual characters safe.

The loader should avoid printing diagnostics to stdout. Loader errors should go
to stderr and include the config path plus the failed phase: import, default
export, serialization, or Rust validation.

## Runtime Validation

The CLI should validate the config at runtime after Deno evaluates
`cefari.config.ts` and before any command uses the result.

TypeScript types and `defineConfig` provide authoring help only. They cannot be
trusted as enforcement because projects may run without editor checking, use
type assertions, import generated data, or compute values dynamically.

Runtime validation should cover both schema and Cefari semantics:

- reject unknown fields;
- require all required fields;
- reject wrong primitive/container types;
- reject invalid `app.projectName` values;
- reject blank app names, identifiers, package names, versions, and paths;
- validate port ranges;
- validate command arrays as non-empty string arrays when present;
- validate path-like fields as relative project paths unless a field explicitly
  permits absolute paths;
- validate version strings with the package/update version rules Cefari uses for
  release artifacts.

Validation errors should report the project-facing config path and field path.
They should not expose the temporary JSON boundary as the user's source of
truth.

## Deno Version Expectations

Cefari should expect Deno `2.8+` for `cefari.config.ts` execution.

When Deno is missing, config-loading commands should fail because the CLI cannot
execute the project config. When Deno is installed but older than `2.8`, the CLI
should warn and continue. The warning should make the support boundary clear
without blocking developers who are not hitting an actual incompatibility.

Example warning:

```text
warning: Cefari expects Deno 2.8+ to load cefari.config.ts; found Deno 2.7.5
```

`cefari doctor` and release-action diagnostics should report the detected Deno
version and whether it is missing, expected, or older than expected.

## Deno Permissions

The first implementation should run the config with explicit permissions:

- `--allow-read=<project root>` so local imports and project files work.
- `--allow-env` so env-driven configuration is possible.
- No network permission by default.
- No write permission by default.
- No subprocess permission by default.

If a project needs broader permissions later, Cefari can add an explicit
`--config-permission` CLI escape hatch. The default path should bias toward
trustworthy, reproducible configuration.

## Data Model

The Rust data model should continue to own the normalized config shape. The
TypeScript input type should mirror that shape using TypeScript naming
conventions.

Recommended TypeScript names:

- `app.projectName`, replacing TOML `app.project_name`
- `app.name`
- `app.identifier`
- `app.icon`
- `app.trayIcon`
- `frontend.dist`
- `frontend.buildCommand`
- `frontend.devCommand`
- `frontend.devPort`
- `daemon.entry`
- `package.productName`
- `package.version`

The Rust structs can use `serde(rename_all = "camelCase")` for the JSON boundary
instead of preserving TOML-oriented snake case.

Required fields should stay required:

- `app.projectName`
- `app.name`
- `app.identifier`
- `frontend.dist`
- `daemon.entry`
- `package.productName`
- `package.version`

Defaults should remain intentionally narrow:

- `frontend.devPort` defaults to `5173`.
- Optional commands remain optional.
- Icons remain optional until Cefari decides app icons are mandatory for
  packaged apps.

## Icons

`app.icon` should represent the packaged application icon. `app.trayIcon` should
represent the OS tray or menu bar icon.

If `app.trayIcon` is omitted, Cefari may derive a tray asset from `app.icon` in a
future packaging step, but the config shape should make the distinction
available now. Tray icons have different constraints from app icons: they are
small, high-contrast, and often template-style assets.

## Error Behavior

The CLI should produce errors that name `cefari.config.ts`, not an internal JSON
or loader artifact.

Examples:

- `project config not found at /path/to/app/cefari.config.ts`
- `Deno is required to load cefari.config.ts but was not found`
- `warning: Cefari expects Deno 2.8+ to load cefari.config.ts; found Deno 2.7.5`
- `failed to execute project config at /path/to/app/cefari.config.ts`
- `project config default export must be an object`
- `project config default export must be JSON-serializable`
- `failed to parse project config at /path/to/app/cefari.config.ts: missing field app.projectName`
- `unknown project config field frontend.dev_server`

Errors should distinguish Deno execution failures from Rust validation failures.
That distinction matters because the fix is different: TypeScript/import/runtime
failure versus invalid Cefari config data.

## Generated Projects

`cefari init` should generate `cefari.config.ts` instead of `cefari.toml`.

Generated projects should also have enough TypeScript resolution support for the
import to work in editors and in Deno. The preferred path is:

- install or reference `@cefari/cli` as the project-local CLI package;
- include a `deno.json` import mapping if Deno needs it for bare package
  resolution;
- keep the config file small and explicit.

The scaffolded config should use the same display name, identifier, project
name, package version, frontend paths, and daemon entry currently generated in
TOML.

## Documentation Changes

Documentation should be rewritten rather than amended with compatibility notes.

Required updates:

- Replace the `cefari.toml` reference with a `cefari.config.ts` reference.
- Update scaffolding docs to list `cefari.config.ts`.
- Update development docs to show `frontend.devCommand`.
- Update build and package docs to show `frontend.buildCommand`, icons, and
  package version in TypeScript.
- Update CLI diagnostics docs to report `cefari.config.ts`.
- Update templates and skill docs after the primary docs are changed.

## Implementation Plan

1. Add the TypeScript config API to `@cefari/cli`.
   - Add `defineConfig`.
   - Add published `.d.ts` files.
   - Add package `exports`.
   - Keep the existing CLI binary working.

2. Add a Deno config loader to the Rust CLI.
   - Add a small loader script that imports the config file and prints JSON.
   - Spawn Deno from `ProjectConfig::load_from_dir`.
   - Detect missing Deno and return a clear config-load error.
   - Parse `deno --version` and warn when the version is older than `2.8`.
   - Map Deno failures into `LoadProjectError`.
   - Deserialize loader stdout into `ProjectConfig`.
   - Run CLI-owned runtime validation before returning the loaded config.

3. Rename and reshape the config model.
   - Load `cefari.config.ts`.
   - Replace TOML parsing with JSON deserialization.
   - Switch the JSON boundary to camelCase.
   - Add `app.trayIcon`.
   - Keep strict unknown-field rejection.
   - Add explicit semantic validation for names, identifiers, versions, ports,
     paths, and command arrays.

4. Update generated projects and templates.
   - Replace generated `cefari.toml`.
   - Update the Vite template.
   - Update smoke projects.
   - Ensure generated projects can resolve `@cefari/cli` in Deno and editors.

5. Update docs and skill docs.
   - Replace TOML examples.
   - Document Deno `2.8+` as the expected version.
   - Document Deno execution, permissions, and serializable default exports.
   - Sync `docs/` into `skills/cefari/docs/`.

6. Update tests.
   - Add loader tests for successful config execution.
   - Add missing file, missing default export, non-object export, non-serializable
     export, unknown field, and invalid field tests.
   - Update CLI integration tests that inspect generated files.
   - Update package tests that rely on app icon and package version fields.

## Acceptance Criteria

- New projects contain `cefari.config.ts`, not `cefari.toml`.
- `cefari dev`, `cefari build`, `cefari package`, `cefari clean`, and
  `cefari info` load `cefari.config.ts`.
- `import { defineConfig } from "@cefari/cli"` works in generated configs.
- Deno executes the config file and emits JSON for Rust validation.
- Missing Deno fails with a clear message.
- Deno older than `2.8` warns and continues.
- `defineConfig` is not required for enforcement; invalid config fails runtime
  validation even if TypeScript accepts it.
- Unknown fields are rejected.
- Missing required fields are rejected.
- Wrong types, blank required values, invalid ports, invalid project names,
  invalid versions, invalid paths, and malformed command arrays are rejected by
  the CLI.
- Config execution does not have network, write, or subprocess permission by
  default.
- The npm `@cefari/cli` package still exposes the `cefari` binary.
- Documentation no longer presents `cefari.toml` as the current config format.

## Open Follow-Up

Decide whether the first implementation should include an explicit CLI escape
hatch for extra Deno config permissions, or wait until a real project needs it.
