import { strict as assert } from "node:assert";
import { isNotarizableArtifact, isReleaseArtifact, isSignableArtifact } from "../src/artifacts.ts";

Deno.test("classifies release artifacts by supported suffix", () => {
  assert.equal(isReleaseArtifact("dist/package/output/My.app"), true);
  assert.equal(isReleaseArtifact("dist/package/output/My.app.tar.gz"), true);
  assert.equal(isReleaseArtifact("dist/package/output/My.txt"), false);
});

Deno.test("classifies signable artifacts by platform", () => {
  assert.equal(isSignableArtifact("My.dmg", "macos"), true);
  assert.equal(isSignableArtifact("My.deb", "linux"), true);
  assert.equal(isSignableArtifact("My.msi", "windows"), true);
  assert.equal(isSignableArtifact("My.dmg", "windows"), false);
});

Deno.test("classifies notarizable artifacts", () => {
  assert.equal(isNotarizableArtifact("My.app"), true);
  assert.equal(isNotarizableArtifact("My.dmg"), true);
  assert.equal(isNotarizableArtifact("My.zip"), false);
});
