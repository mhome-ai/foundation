import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { buildIntegration } from "../src/index.mjs";

function project(t, files = {}) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "integration-build-"));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  const write = (name, content) => {
    fs.mkdirSync(path.dirname(path.join(root, name)), { recursive: true });
    fs.writeFileSync(path.join(root, name), typeof content === "string" ? content : JSON.stringify(content));
  };
  write("package.json", { name: "fixture-functions", private: true, type: "module", dependencies: {} });
  write("package-lock.json", { lockfileVersion: 3, packages: { "": { dependencies: {} } } });
  for (const [name, content] of Object.entries(files)) write(name, content);
  return { root, write, build: (extra = {}) => buildIntegration({ projectRoot: root, functions: ["calculate"], ...extra }) };
}

test("Integration bundles shared modules and ordered arguments without an Automation contract", async (t) => {
  const files = {
    "calculate.ts": "import {add} from './common/math.js'; export default (left:number, right:number)=>({sum:add(left,right)});",
    "common/math.ts": "export const add = (a:number,b:number) => a+b;",
  };
  const a = project(t, files), b = project(t, files);
  const [first] = await a.build();
  assert.deepEqual(await a.build(), await b.build());
  assert.equal(first.function, "calculate");
  assert.equal(first.artifact.contract, undefined);
  assert.equal(first.artifact.sha256, createHash("sha256").update(first.javascript).digest("hex"));
  assert.equal(first.artifact.size, Buffer.byteLength(first.javascript));
  assert.ok(!first.javascript.includes(a.root));
  const module = await import(`data:text/javascript;base64,${Buffer.from(first.javascript).toString("base64")}`);
  assert.deepEqual(module.default(3, 9), { sum: 12 });
  a.write("common/math.ts", "export const add = (a:number,b:number) => a-b;");
  assert.notEqual((await a.build())[0].sourceHash, first.sourceHash);
});

test("bundles locked pure JavaScript dependencies and hashes their consumed bytes", async (t) => {
  const p = project(t, {
    "package.json": { name: "fixture-functions", dependencies: { colors: "1.2.3" } },
    "package-lock.json": { lockfileVersion: 3, packages: {
      "": { dependencies: { colors: "1.2.3" } },
      "node_modules/colors": { version: "1.2.3", resolved: "https://registry.example/colors.tgz", integrity: "sha512-fixture" },
    } },
    "node_modules/colors/package.json": { name: "colors", version: "1.2.3", main: "index.js", types: "index.d.ts" },
    "node_modules/colors/index.d.ts": "export function average(values:number[]):number;",
    "node_modules/colors/index.js": "exports.average = values => values.reduce((a,b)=>a+b,0)/values.length;",
    "calculate.ts": "import {average} from 'colors'; export default (values:number[])=>average(values);",
  });
  const [built] = await p.build();
  const module = await import(`data:text/javascript;base64,${Buffer.from(built.javascript).toString("base64")}`);
  assert.equal(module.default([2, 4]), 3);
  p.write("node_modules/colors/index.js", "exports.average = values => 8;");
  assert.notEqual((await p.build())[0].sourceHash, built.sourceHash);
  p.write("node_modules/colors/package.json", { name: "colors", version: "1.2.4", main: "index.js" });
  await assert.rejects(p.build(), (error) => error.code === "INVALID_DEPENDENCIES");
});

test("compiler does not evaluate source, tsconfig, plugins or package scripts", async (t) => {
  const p = project(t, {
    "calculate.ts": "throw new Error('DO NOT EXECUTE'); export default (x:number)=>x;",
    "tsconfig.json": "not a config",
  });
  p.write("package.json", { name: "fixture-functions", scripts: { prebuild: "exit 42" } });
  assert.match((await p.build())[0].javascript, /DO NOT EXECUTE/);
});

test("SDK stays managed and only the native host port remains external", async (t) => {
  const p = project(t, { "calculate.ts": "import {readArtifact} from '@mhome/script'; export default async (uri:string)=>(await readArtifact(uri)).byteLength;" });
  const [built] = await p.build();
  assert.match(built.javascript, /@mhome\/script\/host/);
  assert.doesNotMatch(built.javascript, /bindings are unavailable/);
  assert.equal(built.artifact.sdkVersion, "0.2.0");
});

test("type errors, non-functions and missing entries fail before producing artifacts", async (t) => {
  const p = project(t);
  for (const [source, code] of [
    ["export default (x:number):number => 'wrong';", "TYPE_CHECK_FAILED"],
    ["export default {value:3};", "INVALID_ENTRY"],
    ["export const value = 3;", "INVALID_ENTRY"],
  ]) {
    p.write("calculate.ts", source);
    await assert.rejects(p.build(), (error) => error.code === code);
  }
  await assert.rejects(p.build({ functions: ["missing"] }), (error) => error.code === "INVALID_SOURCE");
  await assert.rejects(p.build({ functions: ["../calculate"] }), (error) => error.code === "INVALID_SOURCE");
});

test("undeclared filesystem inputs and Node built-ins are rejected", async (t) => {
  const p = project(t, { "secret.ts": "export default 42;" });
  for (const source of [
    "import n from './secret.js'; export default ()=>n;",
    "import fs from 'node:fs'; export default ()=>fs.readFileSync('/tmp/secret');",
    "export default ()=>process.env.SECRET;",
    "export default ()=>document.cookie;",
  ]) {
    p.write("calculate.ts", source);
    await assert.rejects(p.build(), (error) => error.code === "TYPE_CHECK_FAILED");
  }
});

test("symlink escapes, stale lockfiles and unpinned dependencies fail", async (t) => {
  const external = project(t, { "outside.ts": "export const value = 42;" });
  const p = project(t, { "calculate.ts": "import {value} from './common/linked.js'; export default ()=>value;" });
  fs.mkdirSync(path.join(p.root, "common"));
  fs.symlinkSync(path.join(external.root, "outside.ts"), path.join(p.root, "common/linked.ts"));
  await assert.rejects(p.build(), (error) => error.code === "INVALID_IMPORT");
  p.write("package.json", { dependencies: { missing: "1.0.0" } });
  await assert.rejects(p.build(), (error) => error.code === "INVALID_DEPENDENCIES");
});

test("multiple function entries, empty packages and explicit cancellation", async (t) => {
  const p = project(t, { "calculate.ts": "export default ()=>1;", "other.ts": "export default ()=>2;" });
  assert.deepEqual((await p.build({ functions: ["other", "calculate"] })).map((entry) => entry.function), ["calculate", "other"]);
  assert.deepEqual(await p.build({ functions: [] }), []);
  const controller = new AbortController(); controller.abort();
  await assert.rejects(p.build({ signal: controller.signal }), (error) => error.name === "AbortError");
});

test("computed runtime imports cannot bypass the static bundle", async (t) => {
  const p = project(t, { "calculate.ts": "export default async (name:string)=>await import(name);" });
  await assert.rejects(p.build(), (error) => ["TYPE_CHECK_FAILED", "BUNDLE_FAILED"].includes(error.code));
});

test("source identity preserves BOM bytes and oversized or malformed inputs fail explicitly", async (t) => {
  const source = "export default ()=>1;";
  const p = project(t, { "calculate.ts": source });
  const [plain] = await p.build();
  p.write("calculate.ts", `\uFEFF${source}`);
  const [bom] = await p.build();
  assert.notEqual(plain.sourceHash, bom.sourceHash);
  p.write("calculate.ts", " ".repeat(256 * 1024 + 1));
  await assert.rejects(p.build(), (error) => error.code === "INVALID_SOURCE");
  p.write("package.json", "null");
  await assert.rejects(p.build(), (error) => error.code === "INVALID_DEPENDENCIES");
});

function dependencyProject(t, packages, source) {
  const dependencies = Object.fromEntries(Object.keys(packages).map((name) => [name, "1.0.0"]));
  const files = {
    "package.json": { name: "fixture-functions", dependencies },
    "package-lock.json": { lockfileVersion: 3, packages: {
      "": { dependencies },
      ...Object.fromEntries(Object.keys(packages).map((name) => [`node_modules/${name}`, {
        version: "1.0.0", resolved: `https://registry.example/${name}.tgz`, integrity: "sha512-fixture",
      }])),
    } },
    "calculate.ts": source,
  };
  for (const [name, entries] of Object.entries(packages)) {
    files[`node_modules/${name}/package.json`] = { name, version: "1.0.0", main: "index.js", ...entries.manifest };
    for (const [filename, content] of Object.entries(entries.files)) files[`node_modules/${name}/${filename}`] = content;
  }
  return project(t, files);
}

test("scoped packages and separate type packages survive TypeScript candidate probes", async (t) => {
  for (const name of ["colors", "@demo/colors"]) {
    const types = name.startsWith("@") ? "@types/demo__colors" : "@types/colors";
    const p = dependencyProject(t, {
      [name]: { files: { "index.js": "exports.average = values => values.reduce((a,b)=>a+b,0)/values.length;" } },
      [types]: { files: { "index.d.ts": "export function average(values:number[]):number;" } },
    }, `import {average} from '${name}'; export default (values:number[])=>average(values);`);
    const [built] = await p.build();
    const module = await import(`data:text/javascript;base64,${Buffer.from(built.javascript).toString("base64")}`);
    assert.equal(module.default([2, 4]), 3);
    p.write(`node_modules/${name}/package.json`, { name, version: "1.0.1", main: "index.js" });
    await assert.rejects(p.build(), (error) => error.code === "INVALID_DEPENDENCIES");
  }
});

test("unsupported dependency loaders cannot bypass input hashing, import checks or budgets", async (t) => {
  const p = dependencyProject(t, {
    colors: { manifest: { types: "index.d.ts", main: "index.jsx" }, files: {
      "index.d.ts": "export const answer:number;", "index.jsx": "export const answer=1;",
    } },
  }, "import {answer} from 'colors'; export default ()=>answer;");
  for (const source of ["export const answer=1;", "export const answer=name=>import(name);",
    `/*${"x".repeat(4 * 1024 * 1024)}*/ export const answer=2;`]) {
    p.write("node_modules/colors/index.jsx", source);
    await assert.rejects(p.build(), (error) => error.code === "BUNDLE_FAILED"
      && error.message.includes("Unsupported Integration input extension"));
  }
});

test("hoisted transitive dependencies remain bundled and participate in source identity", async (t) => {
  const p = dependencyProject(t, {
    colors: { files: { "index.d.ts": "export {answer} from '@demo/math';", "index.js": "exports.answer=require('@demo/math').answer;" } },
    "@demo/math": { files: { "index.d.ts": "export const answer:number;", "index.js": "exports.answer=1;" } },
  }, "import {answer} from 'colors'; export default ()=>answer;");
  fs.mkdirSync(path.join(p.root, "node_modules/colors/node_modules"));
  const [before] = await p.build();
  p.write("node_modules/@demo/math/index.js", "exports.answer=2;");
  const [after] = await p.build();
  assert.notEqual(before.sourceHash, after.sourceHash);
  assert.notEqual(before.artifact.sha256, after.artifact.sha256);
});
