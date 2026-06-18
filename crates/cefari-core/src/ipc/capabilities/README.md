# IPC Capability Metadata

Each `.rs` file in this directory describes one IPC capability for the
build-time glue generator in `crates/cefari-core/build.rs`.

The files are Rust-shaped metadata, not compiled modules. Keep the format
simple:

```rust
capability! {
    name: files,
    order: 100,
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

Run this check after editing metadata:

```sh
cargo test -p cefari-core
```

That command rebuilds generated IPC glue, rejects duplicate capability names or
top-level variant tags, and verifies the checked-in TypeScript bindings remain
current.
