import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { execFileSync } from "node:child_process";
import test from "node:test";
import { stageScriptPackage } from "../../../scripts/stage-script-package.mjs";

test("packed SDK and builder install independently and replay offline from a prepared cache", (t) => {
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
  // npm ci caches locked dependency archives, not necessarily the registry
  // metadata that npm install needs to resolve a new consumer. Prepare THIS
  // test's cache explicitly; a developer's warm global cache cannot hide gaps.
  const cache = path.join(root, "npm-cache");
  fs.mkdirSync(cache);
  assert.deepEqual(fs.readdirSync(cache), []);
  const env = { ...process.env, NPM_CONFIG_CACHE: cache };
  delete env.npm_config_cache;
  const install = (args) => execFileSync("npm", args, { cwd: consumer, env, stdio: "pipe" });
  install(["install", "--offline=false", "--ignore-scripts", "--no-audit", "--no-fund", ...tarballs]);
  const lock = fs.readFileSync(path.join(consumer, "package-lock.json"), "utf8");
  fs.rmSync(path.join(consumer, "node_modules"), { recursive: true });
  install(["ci", "--offline", "--ignore-scripts", "--no-audit", "--no-fund"]);
  assert.equal(fs.readFileSync(path.join(consumer, "package-lock.json"), "utf8"), lock);
  for (const name of ["script", "script-build"]) {
    assert.equal(fs.lstatSync(path.join(consumer, "node_modules/@mhome", name)).isSymbolicLink(), false);
  }
  const project = path.join(root, "provider");
  fs.mkdirSync(project);
  fs.writeFileSync(path.join(project, "package.json"), '{"private":true,"name":"fixture-provider"}');
  fs.writeFileSync(path.join(project, "package-lock.json"), '{"lockfileVersion":3,"packages":{"":{}}}');
  fs.writeFileSync(path.join(project, "entry.ts"), "import {ScriptError} from '@mhome/script'; export default (x:number) => { if(x<0) throw new ScriptError('BAD_REQUEST','negative'); return x*2; };");
  const output = path.join(root, "output");
  execFileSync(process.execPath, [path.join(consumer, "node_modules/@mhome/script-build/src/cli.mjs"), project, '["entry"]', output], { cwd: consumer, stdio: "pipe" });
  const built = JSON.parse(fs.readFileSync(path.join(output, "build.json"), "utf8"));
  assert.equal(built.functions[0].artifact.format, "javascript-esm");
  assert.ok(fs.readFileSync(path.join(output, "entry.js"), "utf8").includes("BAD_REQUEST"));
});
