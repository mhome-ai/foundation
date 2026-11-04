import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import fs from "node:fs";
import test from "node:test";
import { buildAutomation, BuildError } from "../src/index.mjs";

const sample = `import { defineScript, t } from '@mhome/script';
export default defineScript({
  input: { samples: t.list(t.number), settings: t.object, anything: t.value },
  output: { average: t.number, labels: t.list(t.string) },
  async run({samples, settings, anything}) {
    return { average: samples.reduce((sum,value)=>sum+value,0)/samples.length, labels: ['ready'] };
  }
});`;

test("one declaration produces exact contract, deterministic JavaScript and hash", async () => {
  const a = await buildAutomation({ function: "average", source: sample });
  const b = await buildAutomation({ function: "average", source: sample });
  assert.deepEqual(a, b);
  assert.equal(a.artifact.sha256, createHash("sha256").update(a.javascript).digest("hex"));
  assert.equal(a.artifact.size, Buffer.byteLength(a.javascript));
  assert.deepEqual(a.artifact.contract.input, [{name:"samples", type:{list:"number"}}, {name:"settings",type:"object"}, {name:"anything",type:"value"}]);
  assert.ok(!a.javascript.includes("from\"@mhome/script\""));
  assert.ok(!a.javascript.includes("/Users/"));
  assert.ok(!a.javascript.includes("@mhome/script/host"));
  // Test the built pure result, never run untrusted source inside the build path.
  const module = await import(`data:text/javascript;base64,${Buffer.from(a.javascript).toString("base64")}`);
  assert.deepEqual(await module.default({samples:[2,4],settings:{},anything:null}), {average:3,labels:["ready"]});
});

test("SDK aliases and nested lists match the Rust protocol corpus", async () => {
  const source = `import {defineScript as script, t as types} from '@mhome/script';
  export default script({input:{rows:types.list(types.list(types.number))},output:{rows:types.list(types.list(types.number))},run: input=>input});`;
  const result = await buildAutomation({function:"nested",source});
  assert.deepEqual(result.artifact.contract.input[0].type, {list:{list:"number"}});
  const fixture = JSON.parse(fs.readFileSync(new URL("../../../crates/script-api/fixtures/javascript-v1.json", import.meta.url), "utf8"));
  assert.equal(result.artifact.contract.version, fixture.validContracts[0].version);
  assert.equal(result.artifact.contract.sdkVersion, "0.2.0");
});

test("wrong return types and missing repeated dimensions fail the TS checker", async () => {
  for (const source of [sample.replace("average: samples.reduce((sum,value)=>sum+value,0)/samples.length", "average: 'wrong'"),
    sample.replace("labels: ['ready']", "labels: [123]"),
    sample.replace("labels: ['ready']", "labels: [['ready']]"),
    sample.replace("labels: ['ready']", "extra: 'wrong'")]) {
    await assert.rejects(buildAutomation({function:"bad",source}), (error) => error instanceof BuildError && error.code === "TYPE_CHECK_FAILED" && error.diagnostics.some((d)=>d.line > 0));
  }
});

test("computed metadata and descriptor variables fail without executing code", async () => {
  const sources = [
    `import {defineScript,t} from '@mhome/script';const input={x:t.number};export default defineScript({input,output:{x:t.number},run:x=>x})`,
    `import {defineScript,t} from '@mhome/script';const n=t.number;export default defineScript({input:{x:n},output:{x:t.number},run:x=>x})`,
    `import {defineScript,t} from '@mhome/script';export default defineScript({input:{['x']:t.number},output:{x:t.number},run:x=>x})`,
    `import {defineScript,t} from '@mhome/script';export default defineScript({input:{...{x:t.number}},output:{x:t.number},run:x=>x})`,
  ];
  for (const source of sources) await assert.rejects(buildAutomation({function:"bad",source}), (e)=>e.code === "INVALID_CONTRACT");
  const source = `throw new Error('MODULE MUST NOT RUN WHILE BUILDING');\n${sample}`;
  assert.ok((await buildAutomation({function:"unexecuted",source})).javascript.includes("MODULE MUST NOT RUN WHILE BUILDING"));
});

test("managed build rejects Node, DOM, arbitrary dependencies and local imports", async () => {
  for (const prefix of ["import fs from 'node:fs';", "import x from './secret.ts';", "import x from 'unapproved-package';", "process.env.SECRET;", "document.cookie;",
    `/// <reference path="${new URL("../src/contract.mjs", import.meta.url).pathname}" />\n`,
    `/// <reference types="node" />\n`]) {
    await assert.rejects(buildAutomation({function:"bad",source:prefix+sample}), (e)=>e.code === "TYPE_CHECK_FAILED");
  }
  const source = `import {call} from '@mhome/script/host';import {defineScript,t} from '@mhome/script';
  export default defineScript({input:{},output:{value:t.value},async run(){return {value:await call('echo',[])}}});`;
  assert.ok((await buildAutomation({function:"host",source})).javascript.includes("@mhome/script/host"));
});

test("validates source identity, exact SDK, UTF-8 size and cancellation", async () => {
  for (const name of ["../bad", "name.ts", "", "x".repeat(129)]) await assert.rejects(buildAutomation({function:name,source:sample}), (e)=>e.code === "INVALID_SOURCE");
  await assert.rejects(buildAutomation({function:"bad",source:sample,sdkVersion:"^0.1.0"}), (e)=>e.code === "UNSUPPORTED_SDK");
  await assert.rejects(buildAutomation({function:"bad",source:sample,sdkVersion:"0.1.0"}), (e)=>e.code === "UNSUPPORTED_SDK");
  await assert.rejects(buildAutomation({function:"bad",source:"猫".repeat(100_000)}), (e)=>e.code === "INVALID_SOURCE");
  const abort = new AbortController(); abort.abort();
  await assert.rejects(buildAutomation({function:"bad",source:sample,signal:abort.signal}), (e)=>e.name === "AbortError");
});

test("SDK host helper keeps native import and never bundles the Node fallback", async () => {
  const source = `import {defineScript,t,readArtifact} from '@mhome/script';
  export default defineScript({input:{uri:t.string},output:{size:t.number},
    async run({uri}){return {size:(await readArtifact(uri)).byteLength}}});`;
  const {javascript} = await buildAutomation({function:"read",source});
  assert.ok(javascript.includes("@mhome/script/host"));
  assert.ok(javascript.includes("artifact.read"));
  assert.ok(!javascript.includes("bindings are unavailable"));
});

test("Rust execution fixture matches the actual managed build", async () => {
  const directory = new URL("../../../crates/script-runtime/fixtures/", import.meta.url);
  const source = fs.readFileSync(new URL("lighting.ts", directory), "utf8");
  const { javascript, ...metadata } = await buildAutomation({function:"lighting",source});
  assert.equal(javascript, fs.readFileSync(new URL("lighting.js", directory), "utf8"));
  assert.deepEqual(metadata, JSON.parse(fs.readFileSync(new URL("lighting.build.json", directory), "utf8")));
});
