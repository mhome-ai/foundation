import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import { renderTimeouts, validateTimeouts } from "../generate-operation-timeouts.mjs";

const manifest = JSON.parse(fs.readFileSync(new URL("../../crates/core-api/manifest/operation-timeouts.v1.json", import.meta.url), "utf8"));
test("checked-in Rust bindings match the authoritative catalog", () => {
  assert.equal(renderTimeouts(manifest), fs.readFileSync(new URL("../../crates/core-api/src/timeouts.generated.rs", import.meta.url), "utf8"));
});
test("reject ambiguous, unbounded and overflowing policies", () => {
  for (const mutate of [
    x => x.operations.coreRequest.totalMs = 0,
    x => x.operations.coreRequest.totalMs = -1,
    x => x.operations.coreRequest.totalMs = 1.5,
    x => x.operations.coreRequest.totalMs = 2147483647,
    x => x.waitProfiles.lionForward.overrides.unknown = 10,
    x => x.selectors.sourceScriptSave.push(x.selectors.storedScriptSave[0]),
    x => x.selectors.ordinaryOverrides["/test"] = "unknown",
    x => x.operations.coreRequest.dynamic = true,
  ]) {
    const copy = structuredClone(manifest);
    mutate(copy);
    assert.throws(() => validateTimeouts(copy));
  }
});
