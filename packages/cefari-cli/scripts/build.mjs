import { cp, readFile, readdir, rm, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const repoRoot = resolve(packageRoot, "../..");
const appBuildSource = resolve(packageRoot, "dist/.app-src");

const tsc = spawnSync("tsc", ["-p", "tsconfig.json"], {
  cwd: packageRoot,
  stdio: "inherit",
  shell: process.platform === "win32",
});

if (tsc.error !== undefined) {
  throw tsc.error;
}

if (tsc.status !== 0) {
  process.exit(tsc.status ?? 1);
}

await rm(resolve(packageRoot, "dist/app"), { force: true, recursive: true });
await rm(appBuildSource, { force: true, recursive: true });
await cp(resolve(repoRoot, "packages/cefari-app/src"), appBuildSource, { recursive: true });
await rewriteTypeScriptSpecifiers(appBuildSource);
await writeFile(
  resolve(packageRoot, "dist/app-tsconfig.json"),
  `${JSON.stringify(
    {
      compilerOptions: {
        target: "ES2022",
        module: "NodeNext",
        moduleResolution: "NodeNext",
        lib: ["ES2022", "DOM"],
        rootDir: ".app-src",
        outDir: "app",
        declaration: true,
        sourceMap: true,
        strict: true,
        forceConsistentCasingInFileNames: true,
        skipLibCheck: true,
      },
      include: [".app-src/**/*.ts"],
    },
    null,
    2,
  )}\n`,
);

const appTsc = spawnSync("tsc", ["-p", "dist/app-tsconfig.json"], {
  cwd: packageRoot,
  stdio: "inherit",
  shell: process.platform === "win32",
});

await rm(appBuildSource, { force: true, recursive: true });
await rm(resolve(packageRoot, "dist/app-tsconfig.json"), { force: true });

if (appTsc.error !== undefined) {
  throw appTsc.error;
}

if (appTsc.status !== 0) {
  process.exit(appTsc.status ?? 1);
}

async function rewriteTypeScriptSpecifiers(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  await Promise.all(
    entries.map(async (entry) => {
      const path = resolve(directory, entry.name);
      if (entry.isDirectory()) {
        await rewriteTypeScriptSpecifiers(path);
        return;
      }
      if (!entry.isFile() || !entry.name.endsWith(".ts")) return;
      const source = await readFile(path, "utf8");
      await writeFile(path, source.replaceAll(/(from\s+["'][^"']+)\.ts(["'])/g, "$1.js$2"));
    }),
  );
}
