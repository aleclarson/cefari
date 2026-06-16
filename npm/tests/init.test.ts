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
});
