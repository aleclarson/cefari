import assert from "node:assert/strict";
import test from "node:test";
import { currentPlatform, withPlatformForTest } from "../src/platform.js";

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
