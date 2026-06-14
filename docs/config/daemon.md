# `daemon`

The `daemon` section points Cefari at the Deno daemon entrypoint.

```ts
daemon: {
  entry: "daemon/main.ts",
}
```

## Fields

| Field | Required | Description |
| --- | --- | --- |
| `entry` | Yes | Deno daemon source file used by `cefari dev` and `cefari build`. |

## Development Behavior

`cefari dev` runs the daemon with Deno watch mode from the project root:

```bash
deno run --watch --allow-read --allow-net <entry>
```

## Build Behavior

`cefari build` copies the daemon entry to `build/daemon/main.ts` and compiles it
with Deno into the project-named daemon executable.

The current daemon build grants read and network permissions to the compiled
daemon.
