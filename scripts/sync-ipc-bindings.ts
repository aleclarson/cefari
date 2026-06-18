const root = new URL("../", import.meta.url);
const coreBinding = new URL("crates/cefari-core/bindings/ipc.ts", root);
const npmBinding = new URL("npm/src/app/ipc.ts", root);

const check = Deno.args.length === 1 && Deno.args[0] === "--check";
if (Deno.args.length > 1 || (Deno.args.length === 1 && !check)) {
  console.warn("usage: deno task ipc:sync");
  console.warn("       deno task ipc:check");
  Deno.exit(2);
}

await verifyCoreBinding();

if (check) {
  Deno.exit(await bindingsMatch() ? 0 : 1);
}

await Deno.copyFile(coreBinding, npmBinding);
console.log("synced crates/cefari-core/bindings/ipc.ts -> npm/src/app/ipc.ts");

async function verifyCoreBinding(): Promise<void> {
  const command = new Deno.Command("cargo", {
    args: [
      "test",
      "-p",
      "cefari-core",
      "generated_typescript_bindings_are_current",
    ],
    cwd: root,
    stdout: "inherit",
    stderr: "inherit",
  });
  const status = await command.output();
  if (!status.success) {
    console.error(
      "core IPC binding is stale or failed to build; update crates/cefari-core/bindings/ipc.ts first",
    );
    Deno.exit(status.code);
  }
}

async function bindingsMatch(): Promise<boolean> {
  const [coreBytes, npmBytes] = await Promise.all([
    Deno.readFile(coreBinding),
    Deno.readFile(npmBinding),
  ]);

  if (coreBytes.length === npmBytes.length) {
    const same = coreBytes.every((byte, index) => byte === npmBytes[index]);
    if (same) return true;
  }

  console.error(
    "npm/src/app/ipc.ts is stale; run `deno task ipc:sync` from the repository root",
  );
  return false;
}
