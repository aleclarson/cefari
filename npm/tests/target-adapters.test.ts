import assert from "node:assert/strict";
import test from "node:test";
import { iosSimulatorPlan, runTargetedDev } from "../src/index.js";
import { withPlatformForTest } from "../src/platform.js";

test("describes the planned iOS simulator command path", () => {
  assert.deepEqual(iosSimulatorPlan().commands, [
    ["xcrun", "simctl", "boot", "<device>"],
    ["xcrun", "simctl", "install", "booted", "<cefari-ios-app-bundle>"],
    ["xcrun", "simctl", "launch", "booted", "<bundle-id>"],
  ]);
});

test("reports missing Xcode tooling for iOS dev", async () => {
  await assert.rejects(
    withPlatformForTest(
      {
        spawnSync(command) {
          return {
            status: command === "xcode-select" ? 1 : 0,
          };
        },
      },
      () => runTargetedDev({ target: "ios" }),
    ),
    /Xcode command line tools are required/,
  );
});

test("reports the planned simulator commands after tooling is available", async () => {
  await assert.rejects(
    withPlatformForTest(
      {
        spawnSync() {
          return { status: 0 };
        },
      },
      () => runTargetedDev({ target: "ios" }),
    ),
    /xcrun simctl install booted <cefari-ios-app-bundle>/,
  );
});
