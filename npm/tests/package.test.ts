import { writeFileSync } from "node:fs";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import test from "node:test";
import { runCefariPackage, runPackageSign, runPackageUpdate } from "../src/index.js";
import type { PackageDependencies } from "../src/index.js";

const testDir = dirname(fileURLToPath(import.meta.url));
const configApi = pathToFileURL(resolve(testDir, "../src/index.js")).href;

async function projectWithPackageBuild(): Promise<string> {
  const root = await mkdtemp(join(tmpdir(), "cefari-package-"));
  await mkdir(join(root, "build/frontend"), { recursive: true });
  await mkdir(join(root, "build/config"), { recursive: true });
  await mkdir(join(root, "build/daemon"), { recursive: true });
  await mkdir(join(root, "build/desktop"), { recursive: true });
  await mkdir(join(root, "build/cef/resources"), { recursive: true });
  await mkdir(join(root, "assets"), { recursive: true });
  await writeFile(join(root, "assets/app.png"), "app-icon");
  await writeFile(join(root, "assets/tray.png"), "tray-icon");
  await writeFile(join(root, "build/frontend/index.html"), "<!doctype html>");
  await writeFile(
    join(root, "build/config/cefari.json"),
    JSON.stringify({ app: { identifier: "dev.cefari.package" }, deep_links: { schemes: ["packageapp"] } }),
  );
  await writeFile(join(root, "build/daemon/package-app-daemon"), "daemon");
  await writeFile(join(root, "build/desktop/package-app"), "desktop");
  await writeFile(
    join(root, "build/cef/resources/archive.json"),
    JSON.stringify({ name: "cef.tar.bz2", sha1: "fixture-sha1" }),
  );
  await writeFile(
    join(root, "cefari.config.ts"),
    `import { deepLinks, defineConfig, tray } from "${configApi}";

export default defineConfig({
  app: {
    projectName: "package-app",
    name: "Package App",
    identifier: "dev.cefari.package",
    icon: "assets/app.png",
  },
  capabilities: [
    tray({ icon: "assets/tray.png" }),
    deepLinks({ schemes: ["packageapp", "packageapp+dev"] }),
  ],
  daemon: {
    entry: "daemon/main.ts",
  },
  package: {
    productName: "Package App",
    version: "0.1.0",
  },
});
`,
  );
  return root;
}

function packageDeps(spawned: Array<{ command: string; args: string[] }> = []): PackageDependencies {
  return {
    spawnSync(command, args) {
      spawned.push({ command, args });
      const outputIndex = args.indexOf("--output");
      if (outputIndex !== -1) {
        const output = args[outputIndex + 1];
        if (typeof output === "string") {
          writeFileSync(output, "signature");
        }
      }
      return { status: 0 };
    },
    env: {},
    stdout: {
      write() {
        return true;
      },
    },
  };
}

test("package writes metadata and manifest for build artifacts", async () => {
  const root = await projectWithPackageBuild();
  const spawned: Array<{ command: string; args: string[] }> = [];

  await runCefariPackage({ root, releaseVersion: "1.2.3" }, packageDeps(spawned));

  const metadata = await readFile(join(root, "dist/package/cargo-packager.toml"), "utf8");
  const packageFormat = process.platform === "darwin" ? "dmg" : process.platform === "win32" ? "nsis" : "deb";
  assert.match(metadata, /name = "dev\.cefari\.package"/);
  assert.match(metadata, /version = "1\.2\.3"/);
  assert.match(metadata, new RegExp(`formats = \\["${packageFormat}"\\]`));
  assert.match(metadata, /target = "frontend"/);
  assert.match(metadata, /target = "config"/);
  assert.match(metadata, /target = "tray-icon\.png"/);
  assert.match(metadata, /\[\[deep_link_protocols\]\]/);
  assert.match(metadata, /schemes = \["packageapp", "packageapp\+dev"\]/);
  const manifest = JSON.parse(await readFile(join(root, "dist/package/manifest.json"), "utf8"));
  assert.equal(manifest.product_name, "Package App");
  assert.equal(manifest.tray_icon, "tray-icon.png");
  assert.match(manifest.config_file, /build\/config\/cefari\.json/);
  assert.match(manifest.daemon_executable, /package-app-daemon/);
  assert.deepEqual(spawned[0], {
    command: "cargo-packager",
    args: ["--config", join(root, "dist/package/cargo-packager.toml"), "--out-dir", join(root, "dist/package/output")],
  });
});

test("package sign maps to cargo-codesign macos args", async () => {
  const root = await mkdtemp(join(tmpdir(), "cefari-sign-"));
  const artifact = join(root, "Example.dmg");
  await writeFile(artifact, "dmg");
  const spawned: Array<{ command: string; args: string[] }> = [];

  runPackageSign({ artifact, platform: "macos", config: "sign.toml" }, packageDeps(spawned));

  assert.deepEqual(spawned[0], {
    command: "cargo-codesign",
    args: ["codesign", "--config", "sign.toml", "macos", "--dmg", artifact, "--skip-notarize"],
  });
});

test("package update writes update manifest", async () => {
  const root = await mkdtemp(join(tmpdir(), "cefari-update-"));
  const archive = join(root, "release.tar.gz");
  const outputDir = join(root, "update");
  await writeFile(archive, "archive");
  const spawned: Array<{ command: string; args: string[] }> = [];

  await runPackageUpdate(
    {
      archive,
      url: "https://downloads.example.test/release.tar.gz",
      version: "1.2.3",
      target: "darwin-aarch64",
      format: "app",
      outputDir,
    },
    packageDeps(spawned),
  );

  assert.equal(spawned[0].command, "cargo-codesign");
  const update = JSON.parse(await readFile(join(outputDir, "update.json"), "utf8"));
  assert.equal(update.version, "1.2.3");
  assert.equal(update.platforms["darwin-aarch64"].signature, "signature");
});
