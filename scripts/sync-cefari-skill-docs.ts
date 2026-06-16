const root = new URL("../", import.meta.url);
const sourceDir = new URL("docs/", root);
const targetDir = new URL("skills/cefari/docs/", root);
const excludedFiles = new Set([
  "architecture.md",
  "ipc.md",
  "runtime/notifications.md",
  "typescript/raw-ipc.md",
  "verification.md",
]);

async function relativeFiles(dir: URL): Promise<string[]> {
  const files: string[] = [];

  async function walk(current: URL, prefix = "") {
    for await (const entry of Deno.readDir(current)) {
      const relative = prefix ? `${prefix}/${entry.name}` : entry.name;
      const child = new URL(
        `${entry.name}${entry.isDirectory ? "/" : ""}`,
        current,
      );
      if (entry.isDirectory) {
        await walk(child, relative);
      } else if (entry.isFile && !excludedFiles.has(relative)) {
        files.push(relative);
      }
    }
  }

  await walk(dir);
  return files.sort();
}

async function exists(path: URL): Promise<boolean> {
  return await Deno.stat(path).then(() => true).catch(() => false);
}

async function sameFile(left: URL, right: URL): Promise<boolean> {
  const [leftBytes, rightBytes] = await Promise.all([
    Deno.readFile(left),
    Deno.readFile(right),
  ]);
  if (leftBytes.length !== rightBytes.length) return false;
  return leftBytes.every((byte, index) => byte === rightBytes[index]);
}

function joinUrl(base: URL, relative: string): URL {
  return new URL(relative.split("/").map(encodeURIComponent).join("/"), base);
}

async function checkSync(): Promise<boolean> {
  if (!await exists(targetDir)) {
    console.warn(
      "skills/cefari/docs does not exist. Run scripts/sync-cefari-skill-docs.ts.",
    );
    return false;
  }

  const [sourceFiles, targetFiles] = await Promise.all([
    relativeFiles(sourceDir),
    relativeFiles(targetDir),
  ]);
  let ok = true;

  const extraSource = sourceFiles.filter((path) => !targetFiles.includes(path));
  const extraTarget = targetFiles.filter((path) => !sourceFiles.includes(path));
  const changed: string[] = [];
  for (const path of sourceFiles.filter((path) => targetFiles.includes(path))) {
    if (!await sameFile(joinUrl(sourceDir, path), joinUrl(targetDir, path))) {
      changed.push(path);
    }
  }

  if (extraSource.length) {
    console.warn("Missing files in skills/cefari/docs:");
    extraSource.forEach((path) => console.warn(`  ${path}`));
    ok = false;
  }

  if (extraTarget.length) {
    console.warn("Extra files in skills/cefari/docs:");
    extraTarget.forEach((path) => console.warn(`  ${path}`));
    ok = false;
  }

  if (changed.length) {
    console.warn("Changed files in skills/cefari/docs:");
    changed.forEach((path) => console.warn(`  ${path}`));
    ok = false;
  }

  if (!ok) {
    console.warn(
      "skills/cefari/docs is stale. Run scripts/sync-cefari-skill-docs.ts.",
    );
  }
  return ok;
}

if (Deno.args.length === 1 && Deno.args[0] === "--check") {
  Deno.exit(await checkSync() ? 0 : 1);
} else if (Deno.args.length === 0) {
  await Deno.remove(targetDir, { recursive: true }).catch(() => undefined);
  for (const path of await relativeFiles(sourceDir)) {
    const target = joinUrl(targetDir, path);
    await Deno.mkdir(new URL("./", target), { recursive: true });
    await Deno.copyFile(joinUrl(sourceDir, path), target);
  }
} else {
  console.warn("usage: scripts/sync-cefari-skill-docs.ts [--check]");
  Deno.exit(2);
}
