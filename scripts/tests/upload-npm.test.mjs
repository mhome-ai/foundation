import assert from "node:assert/strict";
import test from "node:test";
import { uploadNpm } from "../upload-npm.mjs";

test("ambiguous npm results keep readback possible without retrying an upload", () => {
  for (const stdout of [undefined, "not JSON", JSON.stringify({ error: { code: "ECONNRESET" } }),
    Buffer.from(JSON.stringify({ error: { code: "E503" } }))]) {
    const calls = [], messages = [];
    uploadNpm("archive.tgz", "proof.json", {
      run(...args) { calls.push(args); throw Object.assign(new Error("upload"), { status: 1, stdout }); },
      log(message) { messages.push(message); },
    });
    assert.equal(calls.length, 1);
    assert.equal(messages.length, 1);
  }
});

test("a command that could not start fails directly", () => {
  const error = Object.assign(new Error("spawn npm ENOENT"), { code: "ENOENT" });
  assert.throws(() => uploadNpm("archive.tgz", "proof.json", { run() { throw error; } }), value => value === error);
});
