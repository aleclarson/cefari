import assert from "node:assert/strict";
import { mkdtemp, readFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { runCefariInit } from "../src/index.js";

test("initializes a project with a dependency-free Vite config", async () => {
  const parent = await mkdtemp(join(tmpdir(), "cefari-init-"));
  const root = join(parent, "sample-app");

  await runCefariInit({ path: root, name: "Sample App" });

  const config = await readFile(join(root, "frontend/vite.config.js"), "utf8");
  assert.equal(config, "export default {};\n");

  const cefariConfig = await readFile(join(root, "cefari.config.ts"), "utf8");
  assert.match(cefariConfig, /icon: "assets\/512x512\.png"/);

  const appIcon = await readFile(join(root, "assets/512x512.png"));
  assert.deepEqual(appIcon.subarray(0, 8), Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]));
  assert.equal(appIcon.readUInt32BE(16), 512);
  assert.equal(appIcon.readUInt32BE(20), 512);
});
