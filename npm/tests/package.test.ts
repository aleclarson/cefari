import { writeFileSync } from "node:fs";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import test from "node:test";
import { runCefariPackage, runPackageSign, runPackageUpdate } from "../src/index.js";
import { withPlatformForTest } from "../src/platform.js";

const testDir = dirname(fileURLToPath(import.meta.url));
const configApi = pathToFileURL(resolve(testDir, "../src/index.js")).href;

async function projectWithPackageBuild(options: { daemon?: boolean; workerNative?: boolean } = { daemon: true }): Promise<string> {
  const root = await mkdtemp(join(tmpdir(), "cefari-package-"));
  await mkdir(join(root, "build/frontend"), { recursive: true });
  await mkdir(join(root, "build/config"), { recursive: true });
  if (options.daemon !== false) {
    await mkdir(join(root, "build/daemon"), { recursive: true });
    await writeFile(join(root, "build/daemon/package-app-daemon"), "daemon");
  }
  await mkdir(join(root, "build/desktop"), { recursive: true });
  await mkdir(join(root, "build/workers/thumbnailer"), { recursive: true });
  if (options.workerNative) {
    await mkdir(join(root, "build/workers/thumbnailer/native/bin"), { recursive: true });
    if (options.daemon !== false) {
      await mkdir(join(root, "build/daemon/native/bin"), { recursive: true });
    }
  }
  await mkdir(join(root, "build/cef/resources"), { recursive: true });
  await mkdir(join(root, "assets"), { recursive: true });
  await writeFile(join(root, "assets/app.png"), "app-icon");
  await writeFile(join(root, "assets/tray.png"), "tray-icon");
  await writeFile(join(root, "build/frontend/index.html"), "<!doctype html>");
  await writeFile(
    join(root, "build/config/cefari.json"),
    JSON.stringify({ app: { identifier: "dev.cefari.package" }, deep_links: { schemes: ["packageapp"] } }),
  );
  await writeFile(join(root, "build/desktop/package-app"), "desktop");
  await writeFile(join(root, "build/workers/thumbnailer/thumbnailer"), "worker");
  if (options.workerNative) {
    await writeFile(join(root, "build/workers/thumbnailer/native/bin/thumb.exe"), "windows-tool");
    if (options.daemon !== false) {
      await writeFile(join(root, "build/daemon/native/bin/thumb.exe"), "windows-tool");
    }
  }
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
  ${options.workerNative ? `nativeResources: {
    "thumb-tool": {
      target: "bin/thumb.exe",
      sources: {
        "windows-x64": "native/windows-x64/thumb.exe",
      },
      executable: true,
    },
    "linux-thumb-tool": {
      target: "bin/thumb",
      sources: {
        "linux-x64": "native/linux-x64/thumb",
      },
      executable: true,
    },
  },` : ""}
  workers: {
    thumbnailer: {
      entry: "workers/thumbnailer.ts",
      permissions: {},
      ${options.workerNative ? `native: ["thumb-tool", "linux-thumb-tool"],` : ""}
    },
  },
  ${options.daemon === false ? "" : `daemon: {
    entry: "daemon/main.ts",
    ${options.workerNative ? `native: ["thumb-tool"],` : ""}
  },`}
  package: {
    productName: "Package App",
    version: "0.1.0",
  },
});
`,
  );
  return root;
}

async function withPackagePlatform<T>(
  spawned: Array<{ command: string; args: string[] }>,
  fn: () => T | Promise<T>,
): Promise<Awaited<T>> {
  return withPlatformForTest(
    {
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
    },
    fn,
  );
}

test("package writes metadata and manifest for build artifacts", async () => {
  const root = await projectWithPackageBuild();
  const spawned: Array<{ command: string; args: string[] }> = [];

  await withPackagePlatform(spawned, async () => {
    await runCefariPackage({ root, releaseVersion: "1.2.3" });
  });

  const metadata = await readFile(join(root, "dist/package/cargo-packager.toml"), "utf8");
  const packageFormat = process.platform === "darwin" ? "dmg" : process.platform === "win32" ? "nsis" : "deb";
  assert.match(metadata, /name = "dev\.cefari\.package"/);
  assert.match(metadata, /identifier = "dev\.cefari\.package"/);
  assert.match(metadata, /version = "1\.2\.3"/);
  assert.match(metadata, new RegExp(`formats = \\["${packageFormat}"\\]`));
  assert.match(metadata, /\[\[deep_link_protocols\]\]/);
  assert.match(metadata, /schemes = \["cefari-notification-dev-cefari-package"\]/);
  assert.match(metadata, /name = "dev\.cefari\.package\.notification"/);
  assert.match(metadata, /target = "frontend"/);
  assert.match(metadata, /target = "config"/);
  assert.match(metadata, /target = "workers"/);
  assert.match(metadata, /target = "tray-icon\.png"/);
  assert.match(metadata, /\[\[deep_link_protocols\]\]/);
  assert.match(metadata, /schemes = \["packageapp", "packageapp\+dev"\]/);
  const manifest = JSON.parse(await readFile(join(root, "dist/package/manifest.json"), "utf8"));
  assert.equal(manifest.product_name, "Package App");
  assert.equal(manifest.identifier, "dev.cefari.package");
  assert.equal(manifest.notification_protocol, "cefari-notification-dev-cefari-package");
  assert.equal(manifest.tray_icon, "tray-icon.png");
  assert.match(manifest.config_file, /build\/config\/cefari\.json/);
  assert.match(manifest.daemon_executable, /package-app-daemon/);
  assert.match(manifest.workers_dir, /build\/workers/);
  assert.match(manifest.worker_executables.thumbnailer, /build\/workers\/thumbnailer\/thumbnailer/);
  assert.deepEqual(spawned[0], {
    command: "cargo-packager",
    args: ["--config", join(root, "dist/package/cargo-packager.toml"), "--out-dir", join(root, "dist/package/output")],
  });
});

test("package omits daemon artifacts when daemon is not configured", async () => {
  const root = await projectWithPackageBuild({ daemon: false });

  await withPackagePlatform([], async () => {
    await runCefariPackage({ root });
  });

  const metadata = await readFile(join(root, "dist/package/cargo-packager.toml"), "utf8");
  assert.doesNotMatch(metadata, /target = "daemon"/);
  const manifest = JSON.parse(await readFile(join(root, "dist/package/manifest.json"), "utf8"));
  assert.equal(Object.hasOwn(manifest, "daemon_dir"), false);
  assert.equal(Object.hasOwn(manifest, "daemon_executable"), false);
});

test("package uses build manifest target for format and executable names", async () => {
  const root = await projectWithPackageBuild();
  await writeFile(join(root, "build/desktop/package-app.exe"), "desktop");
  await writeFile(join(root, "build/daemon/package-app-daemon.exe"), "daemon");
  await writeFile(join(root, "build/workers/thumbnailer/thumbnailer.exe"), "worker");
  await writeFile(
    join(root, "build/cef/manifest.json"),
    JSON.stringify({ target: "windows-x64", target_os: "windows", target_arch: "x64" }),
  );

  await withPackagePlatform([], async () => {
    await runCefariPackage({ root });
  });

  const metadata = await readFile(join(root, "dist/package/cargo-packager.toml"), "utf8");
  assert.match(metadata, /formats = \["nsis"\]/);
  assert.match(metadata, /path = "package-app\.exe"/);
  const manifest = JSON.parse(await readFile(join(root, "dist/package/manifest.json"), "utf8"));
  assert.equal(manifest.desktop_binary, "package-app.exe");
  assert.match(manifest.daemon_executable, /package-app-daemon\.exe/);
  assert.match(manifest.worker_executables.thumbnailer, /thumbnailer\.exe/);
});

test("package includes worker native resources selected by build target", async () => {
  const root = await projectWithPackageBuild({ workerNative: true });
  await writeFile(join(root, "build/desktop/package-app.exe"), "desktop");
  await writeFile(join(root, "build/daemon/package-app-daemon.exe"), "daemon");
  await writeFile(join(root, "build/workers/thumbnailer/thumbnailer.exe"), "worker");
  await writeFile(
    join(root, "build/cef/manifest.json"),
    JSON.stringify({ target: "windows-x64", target_os: "windows", target_arch: "x64" }),
  );

  await withPackagePlatform([], async () => {
    await runCefariPackage({ root });
  });

  const metadata = await readFile(join(root, "dist/package/cargo-packager.toml"), "utf8");
  assert.match(metadata, /target = "workers"/);
  const manifest = JSON.parse(await readFile(join(root, "dist/package/manifest.json"), "utf8"));
  assert.deepEqual(manifest.daemon_native_resources, [
    {
      id: "thumb-tool",
      target: "bin/thumb.exe",
      resource_path: "daemon/native/bin/thumb.exe",
      path: join(root, "build/daemon/native/bin/thumb.exe").replaceAll("\\", "/"),
      executable: true,
    },
  ]);
  assert.deepEqual(manifest.native_resources.thumbnailer, [
    {
      id: "thumb-tool",
      target: "bin/thumb.exe",
      resource_path: "workers/thumbnailer/native/bin/thumb.exe",
      path: join(root, "build/workers/thumbnailer/native/bin/thumb.exe").replaceAll("\\", "/"),
      executable: true,
    },
  ]);
});

test("package ignores worker native resources for other build targets", async () => {
  const root = await projectWithPackageBuild({ workerNative: true });
  await writeFile(
    join(root, "build/cef/manifest.json"),
    JSON.stringify({ target: "darwin-arm64", target_os: "darwin", target_arch: "arm64" }),
  );

  await withPackagePlatform([], async () => {
    await runCefariPackage({ root });
  });

  const manifest = JSON.parse(await readFile(join(root, "dist/package/manifest.json"), "utf8"));
  assert.deepEqual(manifest.native_resources.thumbnailer, []);
});

test("package rejects missing selected worker native resources", async () => {
  const root = await projectWithPackageBuild({ workerNative: true });
  await writeFile(join(root, "build/desktop/package-app.exe"), "desktop");
  await writeFile(join(root, "build/daemon/package-app-daemon.exe"), "daemon");
  await writeFile(join(root, "build/workers/thumbnailer/thumbnailer.exe"), "worker");
  await rm(join(root, "build/workers/thumbnailer/native/bin/thumb.exe"));
  await writeFile(
    join(root, "build/cef/manifest.json"),
    JSON.stringify({ target: "windows-x64", target_os: "windows", target_arch: "x64" }),
  );

  await withPackagePlatform([], async () => {
    await assert.rejects(
      runCefariPackage({ root }),
      /artifact does not exist: .*build\/workers\/thumbnailer\/native\/bin\/thumb\.exe/,
    );
  });
});

test("package rejects missing executables for build manifest target", async () => {
  const root = await projectWithPackageBuild();
  await writeFile(
    join(root, "build/cef/manifest.json"),
    JSON.stringify({ target: "windows-x64", target_os: "windows", target_arch: "x64" }),
  );

  await withPackagePlatform([], async () => {
    await assert.rejects(
      runCefariPackage({ root }),
      /artifact does not exist: .*build\/desktop\/package-app\.exe/,
    );
  });
});

test("package rejects invalid build manifest target", async () => {
  const root = await projectWithPackageBuild();
  await writeFile(
    join(root, "build/cef/manifest.json"),
    JSON.stringify({ target: "freebsd-x64", target_os: "freebsd", target_arch: "x64" }),
  );

  await withPackagePlatform([], async () => {
    await assert.rejects(runCefariPackage({ root }), /build target must be one of/);
  });
});

test("package rejects missing configured worker executables", async () => {
  const root = await projectWithPackageBuild();
  await rm(join(root, "build/workers/thumbnailer/thumbnailer"));

  await withPackagePlatform([], async () => {
    await assert.rejects(
      runCefariPackage({ root }),
      /artifact does not exist: .*build\/workers\/thumbnailer\/thumbnailer/,
    );
  });
});

test("package sign maps to cargo-codesign macos args", async () => {
  const root = await mkdtemp(join(tmpdir(), "cefari-sign-"));
  const artifact = join(root, "Example.dmg");
  await writeFile(artifact, "dmg");
  const spawned: Array<{ command: string; args: string[] }> = [];

  await withPackagePlatform(spawned, () => {
    runPackageSign({ artifact, platform: "macos", config: "sign.toml" });
  });

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

  await withPackagePlatform(spawned, async () => {
    await runPackageUpdate({
      archive,
      url: "https://downloads.example.test/release.tar.gz",
      version: "1.2.3",
      target: "darwin-aarch64",
      format: "app",
      outputDir,
    });
  });

  assert.equal(spawned[0].command, "cargo-codesign");
  const update = JSON.parse(await readFile(join(outputDir, "update.json"), "utf8"));
  assert.equal(update.version, "1.2.3");
  assert.equal(update.platforms["darwin-aarch64"].signature, "signature");
});
