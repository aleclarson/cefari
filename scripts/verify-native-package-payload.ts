const root = Deno.args[0];
if (!root) {
  console.warn(
    "usage: scripts/verify-native-package-payload.ts INSPECT_DIR [RUNNER_OS] [MANIFEST_JSON]",
  );
  Deno.exit(2);
}

const runnerOs = Deno.args[1] ?? Deno.env.get("RUNNER_OS") ?? Deno.build.os;
const manifestPath = Deno.args[2];
const manifest = manifestPath
  ? JSON.parse(await Deno.readTextFile(manifestPath)) as Record<string, string>
  : undefined;

const files: string[] = [];
for await (const path of walkFiles(root)) {
  files.push(path.replaceAll("\\", "/"));
}

function escapeRegex(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function requireMatch(description: string, pattern: RegExp) {
  if (files.some((path) => pattern.test(path))) return;

  console.warn("inspected files:");
  files.forEach((path) => console.warn(`  ${path}`));
  console.error(`native package payload is missing ${description}`);
  Deno.exit(1);
}

function requireManifestString(key: string): string {
  const value = manifest?.[key];
  if (typeof value !== "string") {
    console.error(`manifest is missing ${key}`);
    Deno.exit(1);
  }
  return value;
}

async function* walkFiles(path: string): AsyncGenerator<string> {
  for await (const entry of Deno.readDir(path)) {
    const child = `${path}/${entry.name}`;
    if (entry.isDirectory) {
      yield* walkFiles(child);
    } else if (entry.isFile) {
      yield child;
    }
  }
}

const windows = /windows/i.test(runnerOs);
if (manifestPath && manifest) {
  const desktopBinary = requireManifestString("desktop_binary");
  requireMatch(desktopBinary, new RegExp(`${escapeRegex(desktopBinary)}$`));
} else {
  requireMatch(
    "cefari-desktop",
    windows ? /cefari-desktop\.exe$/ : /cefari-desktop$/,
  );
}
requireMatch("generated frontend", /\/frontend\/index\.html$/);
if (manifest) {
  const daemonExecutable =
    requireManifestString("daemon_executable").split(/[\\/]/).at(-1) ?? "";
  requireMatch(
    daemonExecutable,
    new RegExp(`${escapeRegex(daemonExecutable)}$`),
  );
} else {
  requireMatch(
    "generated daemon",
    windows ? /cefari-daemon\.exe$/ : /cefari-daemon$/,
  );
}
requireMatch("CEF archive metadata", /\/cef\/archive\.json$/);
requireMatch("CEF payload resources", /\/cef\/(?!archive\.json$).+/);
