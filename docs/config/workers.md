# Workers

Configure Deno script workers under the top-level `workers` object:

```ts
export default defineConfig({
  workers: {
    thumbnailer: {
      entry: "workers/thumbnailer.ts",
      permissions: {
        read: ["$appData/uploads"],
        write: ["$appData/cache"],
        net: "none",
      },
    },
  },
});
```

Worker IDs must match `^[a-z][a-z0-9-]*$`. Frontend code uses the ID with
`cefari.workers.spawn()` or `cefari.workers.run()`.

## Entry

`entry` is a project-relative Deno script path. It must stay inside the project.

In development, the desktop runtime launches the entry from the project source
tree. During `cefari build`, Cefari copies the entry directory into
`build/workers/<workerId>/` and writes packaged runtime config that points at
that copied script.

## Permissions

Each worker must declare `permissions`. Supported keys are:

- `read`
- `write`
- `net`
- `env`
- `run`

Each key is either `"none"` or a non-empty string array. Omitted permission keys
default to `"none"`.

Path permissions for `read`, `write`, and `run` accept project-relative paths
or Cefari runtime tokens:

- `$appData`
- `$cache`
- `$resource`

Name permissions for `net` and `env` pass names through to Deno permission
flags. Cefari launches workers with `deno run --no-prompt`, so missing
permissions fail instead of prompting the user.

## Worker Scripts

Worker scripts should use `cefari/worker`:

```ts
import { defineWorker, runCefariWorker } from "cefari/worker";

const worker = defineWorker((init: { cacheDir: string }) => ({
  async thumbnail(
    input: { path: string },
    context: { postMessage(message: { phase: string }): Promise<void> },
  ) {
    await context.postMessage({ phase: "started" });
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
