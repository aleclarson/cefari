# IPC Capability Metadata

Each `.rs` file in this directory describes one IPC capability for the
build-time glue generator in `crates/cefari-core/build.rs`.

The files are Rust-shaped metadata, not compiled modules. Keep the format
simple:

```rust
capability! {
    name: files,
    order: 100,
    support: hostSpecific,
    targets: [desktop, ios, android],
    rationale: "File access is shared at the API level but sandboxed and permission-mediated by host.",
    commands: [
        Files(FilesCommand),
    ],
    results: [
        File(FileResult),
    ],
    events: [
    ],
}
```

Use `event_order` only when event ordering must differ from command/result
ordering to preserve the generated TypeScript contract.

Every capability must declare platform support metadata:

- `support`: one of `portable`, `hostSpecific`, `desktopOnly`, `mobileOnly`, or
  `deferred`.
- `targets`: one or more of `desktop`, `ios`, and `android`.
- `rationale`: a short product-facing explanation of why the capability has that
  support classification.

Run this check after editing metadata:

```sh
cargo test -p cefari-core
```

That command rebuilds generated IPC glue, rejects duplicate capability names or
top-level variant tags, and verifies the checked-in TypeScript bindings remain
current.
