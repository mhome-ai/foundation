import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { defineScript, t, ScriptError, SDK_VERSION, readArtifact, getRuntimeContext,
  requestChannel, sendChannel, isChannelReady, disconnectChannel, getChannelContext,
  browseMdns, requestGoogleCast, log } from "../dist/index.js";

test("declaration is lazy and invocation returns unwrapped business data", async () => {
  let calls = 0;
  const run = defineScript({
    input: { samples: t.list(t.number) }, output: { average: t.number },
    run: async ({ samples }) => {
      calls += 1;
      return { average: samples.reduce((a, b) => a + b, 0) / samples.length };
    },
  });
  assert.equal(calls, 0);
  assert.deepEqual(await run({ samples: [10, 20] }), { average: 15 });
  assert.equal(calls, 1);
});

test("business exceptions propagate without fake success or compensation", async () => {
  const error = new ScriptError("PROVIDER_ERROR", "Device refused the command");
  const run = defineScript({ input: {}, output: {}, run: async () => { throw error; } });
  await assert.rejects(run({}), (value) => value === error && value.code === "PROVIDER_ERROR");
});

test("SDK descriptors match canonical metadata, including nested lists", async () => {
  const fixture = JSON.parse(await readFile(new URL("../../../crates/script-api/fixtures/javascript-v1.json", import.meta.url), "utf8"));
  const contract = fixture.validContracts[0];
  assert.equal(contract.sdkVersion, SDK_VERSION);
  const input = { samples: t.list(t.number), settings: t.object, optionalValue: t.value,
    matrix: t.list(t.list(t.number)), temperature: t.temperature, delay: t.duration,
    at: t.timestamp, photo: t.image };
  assert.deepEqual(Object.entries(input).map(([name, type]) => ({ name, type })), contract.input);
  assert.equal(Object.isFrozen(t), true);
  assert.equal(Object.isFrozen(input.matrix), true);
});

test("author tooling cannot accidentally execute privileged host operations", async () => {
  await assert.rejects(readArtifact("artifact://fictional/image"), /host bindings are unavailable/);
  await assert.rejects(getRuntimeContext(), /host bindings are unavailable/);
  for (const operation of [requestChannel, sendChannel, isChannelReady, disconnectChannel]) {
    await assert.rejects(operation({id: "runtime-id", payload: null}), /host bindings are unavailable/);
  }
  await assert.rejects(getChannelContext(), /host bindings are unavailable/);
  await assert.rejects(browseMdns({serviceTypes: ["_googlecast._tcp.local."]}), /host bindings are unavailable/);
  await assert.rejects(requestGoogleCast({host: "example.test", action: "getStatus"}), /host bindings are unavailable/);
  await assert.rejects(log("fixture"), /host bindings are unavailable/);
});
