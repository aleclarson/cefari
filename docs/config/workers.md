# Workers

Configure Deno source workers under the top-level `workers` object:

```ts
export default defineConfig({
  workers: {
    thumbnailer: {
      entry: "workers/thumbnailer.ts",
      permissions: {
        read: ["$appData/uploads"],
        write: ["$appData/cache"],
        run: ["$resource/workers/thumbnailer/native/bin/thumbnail"],
        ffi: ["$resource/workers/thumbnailer/native/lib/libthumb.dylib"],
        net: "none",
      },
      native: [
        {
          src: "native/darwin-arm64/thumbnail",
          target: "bin/thumbnail",
          platforms: ["darwin-arm64"],
          executable: true,
        },
        {
          src: "native/darwin-arm64/libthumb.dylib",
          target: "lib/libthumb.dylib",
          platforms: ["darwin-arm64"],
        },
      ],
    },
  },
});
```

Worker IDs must match `^[a-z][a-z0-9-]*$`. Frontend code uses the ID with
`cefari.workers.spawn()` or `cefari.workers.run()`.

## Entry

`entry` is a project-relative Deno script path. It must stay inside the project.

In development, the desktop runtime launches the entry from the project source
tree with `deno run --no-prompt`.

During `cefari build`, Cefari compiles each configured worker with
`deno compile` and writes the executable to
`build/workers/<workerId>/<workerExecutableName>`. Packaged runtime config
points at that executable instead of the source entry.

On Windows, worker executables use `.exe`. On other platforms, the executable
name is the worker ID.

## Permissions

Each worker must declare `permissions`. Supported keys are:

- `read`
- `write`
- `net`
- `env`
- `run`
- `ffi`

Each key is either `"none"` or a non-empty string array. Omitted permission keys
default to `"none"`.

Path permissions for `read`, `write`, `run`, and `ffi` accept
project-relative paths or Cefari runtime tokens:

- `$appData`
- `$cache`
- `$resource`

Name permissions for `net` and `env` pass names through to Deno permission
flags.

In development, Cefari launches workers with `deno run --no-prompt`, so missing
permissions fail instead of prompting the user. During build, Cefari passes the
same configured permissions to `deno compile`; packaged worker permissions are
baked into the compiled executable. Changing worker permissions requires
running `cefari build` again.

Runtime path tokens such as `$appData`, `$cache`, and `$resource` refer to
paths that are only known after installation. For compiled workers, those token
permissions are compiled as category-level Deno permissions such as
`--allow-read`.

`ffi` enables Deno FFI for dynamic libraries. Native libraries execute outside
Deno's JavaScript sandbox, so `ffi` should only be granted to libraries the app
author intentionally bundles and trusts.

## Native Payloads

Workers can declare native executables or dynamic libraries in `native`. Native
payloads are owned by the worker that declares them and are packaged separately
from the compiled Deno worker executable.

Each native payload supports:

| Field | Required | Description |
| --- | --- | --- |
| `src` | Yes | Project-relative source file to copy into the package. |
| `target` | Yes | Worker-native resource path used inside the package. |
| `platforms` | No | Cefari build targets that should include this payload. Omit to include the payload for every target. |
| `executable` | No | Whether Cefari should preserve or set executable mode for this payload. Defaults to `false`. |

`src` must stay inside the project. `target` must be a relative resource path
and cannot use parent traversal. Native payload selection uses the effective
`CefariBuildTarget` from `cefari build --target`; cross-target release jobs
should run one build/package pass per target.

Deno compilation does not bundle external native executables or dynamic
libraries automatically. Apps that use `Deno.Command` or `Deno.dlopen` must
declare those native files as worker native payloads and grant the corresponding
`run` or `ffi` permission.

## Worker Scripts

Worker scripts should use `cefari/worker`:

```ts
import {
  defineWorker,
  runCefariWorker,
  workerNativePath,
} from "cefari/worker";

const worker = defineWorker((init: { cacheDir: string }) => ({
  async thumbnail(
    input: { path: string },
    context: { postMessage(message: { phase: string }): Promise<void> },
  ) {
    await context.postMessage({ phase: "started" });
    const thumbnailTool = workerNativePath("bin/thumbnail");
    const command = new Deno.Command(thumbnailTool, {
      args: [input.path, `${init.cacheDir}/thumbnail.png`],
    });
    const result = await command.output();
    if (!result.success) {
      throw new Error("thumbnail tool failed");
    }
    await Deno.readTextFile(input.path);
    return { outputPath: `${init.cacheDir}/thumbnail.png` };
  },
}));

if (import.meta.main) {
  Deno.exit(await runCefariWorker(worker));
}
```

`defineWorker()` infers the worker init input, method inputs, method outputs,
and method messages used by generated frontend worker registry types.

`workerNativePath(target)` returns the absolute path for a configured native
payload target. In development it resolves to the configured source file. In a
packaged app it resolves to the copied package resource. Use
`getWorkerResources()` from `cefari/worker` when a worker needs the resource
directory, native payload directory, or the full native payload target map.

Prefer `Deno.Command` for bundled executable tools and `Deno.dlopen` for
app-owned dynamic libraries with a stable ABI. Cefari does not provide special
NAPI packaging support; native addons must still be packaged as explicit worker
native payloads and loaded by worker code.
