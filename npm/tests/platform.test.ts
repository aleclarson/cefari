import assert from "node:assert/strict";
import test from "node:test";
import {
  cefariBuildTargetInfo,
  currentPlatform,
  executableNameForTarget,
  parseCefariBuildTarget,
  withPlatformForTest,
} from "../src/platform.js";

test("withPlatformForTest restores the previous platform after success", async () => {
  const original = currentPlatform();
  const writes: string[] = [];

  await withPlatformForTest(
    {
      stdout: {
        write(chunk: string | Uint8Array) {
          writes.push(String(chunk));
          return true;
        },
      },
    },
    async () => {
      assert.notEqual(currentPlatform(), original);
      currentPlatform().stdout.write("inside");
    },
  );

  assert.deepEqual(writes, ["inside"]);
  assert.equal(currentPlatform(), original);
});

test("withPlatformForTest restores the previous platform after thrown errors", async () => {
  const original = currentPlatform();

  await assert.rejects(
    withPlatformForTest(
      {
        env: { CEFARI_TEST_MARKER: "override" },
      },
      async () => {
        assert.equal(currentPlatform().env.CEFARI_TEST_MARKER, "override");
        throw new Error("boom");
      },
    ),
    /boom/,
  );

  assert.equal(currentPlatform(), original);
});

test("withPlatformForTest restores nested overrides to the parent override", async () => {
  const original = currentPlatform();

  await withPlatformForTest(
    {
      env: { CEFARI_TEST_MARKER: "outer" },
    },
    async () => {
      const outer = currentPlatform();
      assert.equal(outer.env.CEFARI_TEST_MARKER, "outer");

      await withPlatformForTest(
        {
          env: { CEFARI_TEST_MARKER: "inner" },
        },
        async () => {
          assert.equal(currentPlatform().env.CEFARI_TEST_MARKER, "inner");
        },
      );

      assert.equal(currentPlatform(), outer);
      assert.equal(currentPlatform().env.CEFARI_TEST_MARKER, "outer");
    },
  );

  assert.equal(currentPlatform(), original);
});

test("parses supported Cefari build targets", () => {
  assert.equal(parseCefariBuildTarget("darwin-arm64"), "darwin-arm64");
  assert.equal(parseCefariBuildTarget("linux-x64"), "linux-x64");
  assert.equal(parseCefariBuildTarget("windows-arm64"), "windows-arm64");
  assert.throws(() => parseCefariBuildTarget("freebsd-x64"), /build target must be one of/);
});

test("maps Cefari build targets to Deno targets and executable suffixes", () => {
  assert.deepEqual(cefariBuildTargetInfo("darwin-arm64"), {
    target: "darwin-arm64",
    os: "darwin",
    arch: "arm64",
    denoTarget: "aarch64-apple-darwin",
    executableSuffix: "",
  });
  assert.deepEqual(cefariBuildTargetInfo("linux-x64"), {
    target: "linux-x64",
    os: "linux",
    arch: "x64",
    denoTarget: "x86_64-unknown-linux-gnu",
    executableSuffix: "",
  });
  assert.deepEqual(cefariBuildTargetInfo("windows-x64"), {
    target: "windows-x64",
    os: "windows",
    arch: "x64",
    denoTarget: "x86_64-pc-windows-msvc",
    executableSuffix: ".exe",
  });
  assert.equal(executableNameForTarget("worker", "windows-x64"), "worker.exe");
  assert.equal(executableNameForTarget("worker", "linux-arm64"), "worker");
});
