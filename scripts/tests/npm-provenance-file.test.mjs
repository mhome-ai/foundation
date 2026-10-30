import assert from "node:assert/strict";
import { execFileSync, spawn } from "node:child_process";
import fs from "node:fs";
import http from "node:http";
import os from "node:os";
import path from "node:path";
import test from "node:test";

const supportedNpm = execFileSync("npm", ["--version"], { encoding: "utf8" }).trim() === "12.2.1";

test("the pinned npm client honors the supplied bundle during trusted publishing", { skip: !supportedNpm, timeout: 15000 }, async () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "foundation-npm-proof-"));
  const requests = [];
  const server = http.createServer(async (req, res) => {
    for await (const chunk of req) { /* consume mock OIDC requests */ }
    requests.push(`${req.method} ${req.url}`);
    res.setHeader("Content-Type", "application/json");
    if (req.method === "POST" && req.url.includes("/oidc/token/exchange/")) {
      res.end(JSON.stringify({ token: "test-exchanged-token" }));
    } else if (req.url.includes("/visibility")) {
      res.end(JSON.stringify({ public: true }));
    } else {
      res.end(JSON.stringify({ name: "@mhome/core-protocol", versions: {} }));
    }
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const registry = `http://127.0.0.1:${server.address().port}/`;
  fs.writeFileSync(path.join(root, "package.json"), JSON.stringify({
    name: "@mhome/core-protocol", version: "0.0.1", publishConfig: { access: "public" },
  }));
  fs.writeFileSync(path.join(root, "bundle.json"), "invalid provenance JSON");
  fs.writeFileSync(path.join(root, "npmrc"), "");
  // The test must never reach a public registry, Fulcio or Rekor. A regression
  // that attempts automatic signing fails locally instead of writing anything.
  fs.writeFileSync(path.join(root, "network.cjs"), `
for (const moduleName of ['node:http', 'node:https']) {
  const transport = require(moduleName);
  for (const name of ['request', 'get']) {
    const original = transport[name];
    transport[name] = function (url, ...args) {
      const target = typeof url === 'string' ? new URL(url) : url;
      const host = target.hostname || target.host;
      if (host !== '127.0.0.1') throw new Error('External network blocked by provenance test: ' + host);
      return original.call(this, url, ...args);
    };
  }
}
`);
  const childEnv = { ...process.env };
  for (const key of Object.keys(childEnv)) {
    if (/^npm_config_/i.test(key) || /^(NODE_AUTH_TOKEN|NPM_TOKEN|ACTIONS_ID_TOKEN_|SIGSTORE_ID_TOKEN)/.test(key)) delete childEnv[key];
  }
  Object.assign(childEnv, {
    GITHUB_ACTIONS: "true",
    NPM_ID_TOKEN: `header.${Buffer.from(JSON.stringify({ repository_visibility: "public" })).toString("base64url")}.signature`,
    NODE_OPTIONS: `--require ${path.join(root, "network.cjs")}`,
    npm_config_userconfig: path.join(root, "npmrc"),
    npm_config_globalconfig: path.join(root, "global-npmrc"),
    npm_config_cache: path.join(root, "cache"),
    npm_config_update_notifier: "false",
  });
  try {
    const child = spawn("npm", ["publish", root, "--registry", registry, "--access", "public", "--tag", "latest", "--ignore-scripts", "--provenance-file", path.join(root, "bundle.json")], {
      cwd: root, env: childEnv, stdio: ["ignore", "pipe", "pipe"],
    });
    let output = "";
    child.stdout.on("data", (data) => { output += data; });
    child.stderr.on("data", (data) => { output += data; });
    const timer = setTimeout(() => child.kill("SIGKILL"), 10000);
    let status;
    try { status = await new Promise((resolve, reject) => { child.once("error", reject); child.once("close", resolve); }); }
    finally { clearTimeout(timer); }
    assert.notEqual(status, 0);
    assert.match(output, /Invalid provenance provided/);
    assert.ok(requests.some((request) => request.startsWith("POST ") && request.includes("/oidc/token/exchange/")), JSON.stringify(requests));
    assert.ok(!requests.some((request) => request.startsWith("PUT ")), "invalid proof must prevent package upload");
  } finally {
    await new Promise((resolve) => server.close(resolve));
    fs.rmSync(root, { recursive: true, force: true });
  }
});
