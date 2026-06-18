# `daemon`

The `daemon` section is optional. Omit it for apps that do not need a daemon.
When present, it points Cefari at the Deno daemon entrypoint.

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

`cefari dev` passes the daemon entry to the desktop runtime. The runtime starts
the daemon when frontend code opens a daemon stream.

```bash
deno run -A <entry>
```

The daemon receives `CEFARI_DAEMON=1` and uses stdio for stream traffic. Write
logs to stderr; stdout is reserved for daemon-to-webview bytes.

## Build Behavior

When `daemon` is configured, `cefari build` copies the daemon entry to
`build/daemon/main.ts` and compiles it with Deno into the project-named daemon
executable.

When `daemon` is omitted, Cefari skips `build/daemon/`, writes runtime config
with daemon disabled, and package assembly omits daemon resources.

The current daemon build grants read and network permissions to the compiled
daemon.

## Frontend Streams

Frontend code connects through `cefari.daemon.connect()` from `cefari/app`:

```ts
import { cefari } from "cefari/app";

const connection = await cefari.daemon.connect();
await connection.writable.getWriter().write(new TextEncoder().encode("ping"));
```

The returned `readable` is daemon-to-webview data. The returned `writable` is
webview-to-daemon data. V1 uses stdio internally, but the public API does not
accept transport choices such as HTTP or WebSocket.

Daemon code can use the daemon-side helper:

```ts
import { connect, isCefariDaemon } from "cefari/daemon";

if (isCefariDaemon()) {
  const connection = connect();
  await connection.readable.pipeTo(connection.writable);
}
```
