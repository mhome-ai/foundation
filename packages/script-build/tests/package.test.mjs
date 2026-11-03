import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { execFileSync } from "node:child_process";
import test from "node:test";
import { stageScriptPackage } from "../../../scripts/stage-script-package.mjs";

test("packed SDK and builder install offline without sibling source or lifecycle scripts", (t) => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "standalone-script-packages-"));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  const tarballs = [];
  for (const kind of ["sdk", "build"]) {
    const staging = path.join(root, kind);
    const manifest = stageScriptPackage(kind, staging);
    assert.equal(manifest.private, undefined);
    assert.equal(manifest.scripts, undefined);
    assert.ok(!JSON.stringify(manifest).includes("file:"));
    const packed = JSON.parse(execFileSync("npm", ["pack", staging, "--json", "--ignore-scripts", "--pack-destination", root], { encoding: "utf8" }));
    const result = Array.isArray(packed) ? packed[0] : Object.values(packed)[0];
    tarballs.push(path.join(root, result.filename));
    fs.rmSync(staging, { recursive: true });
  }
  const consumer = path.join(root, "consumer");
  fs.mkdirSync(consumer);
  fs.writeFileSync(path.join(consumer, "package.json"), '{"private":true,"type":"module"}');
  execFileSync("npm", ["install", "--offline", "--ignore-scripts", "--no-audit", "--no-fund", ...tarballs], { cwd: consumer, stdio: "pipe" });
  for (const name of ["script", "script-build"]) {
    assert.equal(fs.lstatSync(path.join(consumer, "node_modules/@meowlink", name)).isSymbolicLink(), false);
  }
  const project = path.join(root, "provider");
  fs.mkdirSync(project);
  fs.writeFileSync(path.join(project, "package.json"), '{"private":true,"name":"fixture-provider"}');
  fs.writeFileSync(path.join(project, "package-lock.json"), '{"lockfileVersion":3,"packages":{"":{}}}');
  fs.writeFileSync(path.join(project, "entry.ts"), "import {ScriptError} from '@meowlink/script'; export default (x:number) => { if(x<0) throw new ScriptError('BAD_REQUEST','negative'); return x*2; };");
  const output = path.join(root, "output");
  execFileSync(process.execPath, [path.join(consumer, "node_modules/@meowlink/script-build/src/cli.mjs"), project, '["entry"]', output], { cwd: consumer, stdio: "pipe" });
  const built = JSON.parse(fs.readFileSync(path.join(output, "build.json"), "utf8"));
  assert.equal(built.functions[0].artifact.format, "javascript-esm");
  assert.ok(fs.readFileSync(path.join(output, "entry.js"), "utf8").includes("BAD_REQUEST"));
});
