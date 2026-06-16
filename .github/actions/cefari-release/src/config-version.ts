import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import type { ReleaseConfig } from "./inputs.ts";

export async function readProjectPackageVersion(config: ReleaseConfig): Promise<string> {
  const loaderDir = await Deno.makeTempDir({ prefix: "cefari-release-config-" });
  try {
    const apiModule = join(loaderDir, "cefari-cli-config-api.ts");
    const importMap = join(loaderDir, "import_map.json");
    await Deno.writeTextFile(
      apiModule,
      [
        "export function defineConfig(config) { return config; }",
        'export function tray(config = {}) { return { type: "tray", ...config }; }',
        "",
      ].join("\n"),
    );
    await Deno.writeTextFile(
      importMap,
      JSON.stringify({ imports: { cefari: pathToFileURL(apiModule).href } }, null, 2),
    );

    const script = [
      'import { pathToFileURL } from "node:url";',
      "const configExport = (await import(pathToFileURL(Deno.args[0]).href)).default;",
      "const context = { command: 'package', packageCommand: 'release', mode: 'production', root: Deno.args[1] };",
      "const config = typeof configExport === 'function' ? await configExport(context) : configExport;",
      "const version = config?.package?.version;",
      'if (typeof version === "string") console.log(version);',
    ].join("\n");

    const command = new Deno.Command("deno", {
      args: [
        "run",
        "-A",
        "--quiet",
        "--import-map",
        importMap,
        "-",
        join(config.projectPath, "cefari.config.ts"),
        resolve(config.projectPath),
      ],
      stdin: "piped",
      stdout: "piped",
      stderr: "inherit",
    });
    const child = command.spawn();
    const writer = child.stdin.getWriter();
    await writer.write(new TextEncoder().encode(script));
    await writer.close();
    const output = await child.output();
    if (!output.success) {
      throw new Error(`failed to read package.version from cefari.config.ts`);
    }
    return new TextDecoder().decode(output.stdout).trim();
  } finally {
    await Deno.remove(loaderDir, { recursive: true }).catch(() => undefined);
  }
}

export async function readPackageMetadataVersion(config: ReleaseConfig): Promise<string> {
  const metadata = join(config.packageDir, "cargo-packager.toml");
  const content = await Deno.readTextFile(metadata).catch((error) => {
    if (error instanceof Deno.errors.NotFound) {
      throw new Error(`package metadata not found at ${metadata}`);
    }
    throw error;
  });
  const match = content.match(/^[ \t]*version[ \t]*=[ \t]*"([^"]+)"/m);
  return match?.[1] ?? "";
}
