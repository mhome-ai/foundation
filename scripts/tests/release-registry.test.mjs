import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import test from "node:test";

test("registry presence, deadlines and package equivalence fail safely", () => {
  const result = spawnSync("python3", ["-B", fileURLToPath(new URL("./test_release_registry.py", import.meta.url))], {
    encoding: "utf8", timeout: 30000,
  });
  assert.equal(result.status, 0, result.stdout + result.stderr);
});
