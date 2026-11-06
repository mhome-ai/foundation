import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash, generateKeyPairSync, sign, verify } from "node:crypto";
import fs from "node:fs";
import http from "node:http";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { protocolBundleBuilder, protocolProvenance, protocolStatement } from "../protocol-provenance.mjs";
import { registryPythonFixture } from "./helpers/registry-python-fixture.mjs";

const env = {
  GITHUB_ACTIONS: "true",
  GITHUB_SERVER_URL: "https://github.com",
  GITHUB_REPOSITORY: "mhome-ai/foundation",
  GITHUB_REF: "refs/tags/mhome-core-api-v1.57.0",
  GITHUB_SHA: "a".repeat(40),
  GITHUB_WORKFLOW_REF: "mhome-ai/foundation/.github/workflows/publish-crate.yml@refs/tags/mhome-core-api-v1.57.0",
  GITHUB_RUN_ID: "123", GITHUB_RUN_ATTEMPT: "2", RUNNER_ENVIRONMENT: "github-hosted",
  GITHUB_EVENT_NAME: "push", GITHUB_REPOSITORY_ID: "456", GITHUB_REPOSITORY_OWNER_ID: "789",
};
const name = "@mhome/core-protocol";
const version = "1.57.0";
const tarball = Buffer.from("final protocol tarball bytes");

test("provenance binds the exact package bytes, tag, source commit and workflow attempt", () => {
  const statement = protocolStatement(name, version, tarball, env);
  assert.equal(statement.subject[0].name, "pkg:npm/%40mhome/core-protocol@1.57.0");
  assert.equal(statement.subject[0].digest.sha512, createHash("sha512").update(tarball).digest("hex"));
  assert.notEqual(protocolStatement(name, version, Buffer.from("changed"), env).subject[0].digest.sha512, statement.subject[0].digest.sha512);
  assert.equal(statement.predicate.buildDefinition.externalParameters.workflow.ref, env.GITHUB_REF);
  assert.deepEqual(statement.predicate.buildDefinition.resolvedDependencies[0].digest, { gitCommit: env.GITHUB_SHA });
  assert.match(statement.predicate.runDetails.metadata.invocationId, /\/runs\/123\/attempts\/2$/);
});

test("untrusted or incomplete workflow context cannot produce release provenance", () => {
  for (const patch of [
    { GITHUB_SHA: "" }, { GITHUB_SHA: "main" }, { RUNNER_ENVIRONMENT: "self-hosted" },
    { GITHUB_REF: "refs/heads/main" }, { GITHUB_WORKFLOW_REF: "other/workflow@tag" },
  ]) assert.throws(() => protocolStatement(name, version, tarball, { ...env, ...patch }));
  assert.throws(() => protocolStatement("@other/package", version, tarball, env));
  assert.throws(() => protocolStatement(name, "latest", tarball, env));
});

async function withRekor({ status = 409, location = true, missing = false } = {}, run) {
  const calls = [];
  const uuid = "b".repeat(80);
  const entry = {
    body: Buffer.from(JSON.stringify({ kind: "dsse", apiVersion: "0.0.1" })).toString("base64"),
    integratedTime: 1700000000, logID: "c".repeat(64), logIndex: 123,
    verification: { signedEntryTimestamp: Buffer.from("test promise").toString("base64") },
  };
  const server = http.createServer(async (req, res) => {
    for await (const chunk of req) { /* consume the signed request */ }
    calls.push(`${req.method} ${req.url}`);
    res.setHeader("Content-Type", "application/json");
    if (req.method === "POST") {
      res.statusCode = status;
      if (status === 409 && location) res.setHeader("Location", `/api/v1/log/entries/${uuid}`);
      res.end(JSON.stringify(status === 201 ? { [uuid]: entry } : { code: status, message: "test registration response" }));
    } else {
      res.statusCode = missing ? 404 : 200;
      res.end(JSON.stringify(missing ? { code: 404, message: "missing proof" } : { [uuid]: entry }));
    }
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const { privateKey, publicKey } = generateKeyPairSync("ec", { namedCurve: "prime256v1" });
  const builder = protocolBundleBuilder({
    signer: { sign: async (data) => ({
      signature: sign("sha256", data, privateKey),
      key: { $case: "publicKey", publicKey: publicKey.export({ type: "spki", format: "pem" }), hint: "test key" },
    }) },
    rekorBaseURL: `http://127.0.0.1:${server.address().port}`,
  });
  try { await run({ builder, calls, uuid, publicKey }); }
  finally { await new Promise((resolve) => server.close(resolve)); }
}

for (const status of [201, 409]) {
  test(`Rekor ${status} retains a signed bundle and requires verification before returning it`, async () => {
    await withRekor({ status }, async ({ builder, calls, uuid, publicKey }) => {
      let verified = false;
      const bundle = await protocolProvenance(name, version, tarball, {
        env, builder,
        verifyBundle: async (signed, policy) => {
          const envelope = signed.dsseEnvelope;
          const payload = Buffer.from(envelope.payload, "base64");
          const pae = Buffer.concat([Buffer.from(`DSSEv1 ${envelope.payloadType.length} ${envelope.payloadType} ${payload.length} `), payload]);
          assert.equal(verify("sha256", pae, publicKey, Buffer.from(envelope.signatures[0].sig, "base64")), true);
          assert.deepEqual(JSON.parse(payload), protocolStatement(name, version, tarball, env));
          const identity = new RegExp(policy.certificateIdentityURI);
          assert.ok(identity.test(`https://github.com/${env.GITHUB_WORKFLOW_REF}`));
          assert.ok(!identity.test(`https://githubXcom/${env.GITHUB_WORKFLOW_REF}`));
          assert.ok(!identity.test(`https://github.com/${env.GITHUB_WORKFLOW_REF}-other`));
          assert.equal(policy.certificateIssuer, "https://token.actions.githubusercontent.com");
          assert.equal(policy.tlogThreshold, 1);
          assert.equal(policy.ctLogThreshold, 1);
          verified = true;
        },
      });
      assert.equal(verified, true);
      assert.equal(bundle.verificationMaterial.tlogEntries.length, 1);
      assert.deepEqual(calls, ["POST /api/v1/log/entries", ...(status === 409 ? [`GET /api/v1/log/entries/${uuid}`] : [])]);
    });
  });
}

for (const [options, code] of [
  [{ status: 403 }, "TLOG_CREATE_ENTRY_ERROR"],
  [{ location: false }, "TLOG_CREATE_ENTRY_ERROR"],
  [{ missing: true }, "TLOG_FETCH_ENTRY_ERROR"],
]) {
  test(`Rekor failures remain fatal: ${JSON.stringify(options)}`, async () => {
    await withRekor(options, async ({ builder }) => {
      await assert.rejects(protocolProvenance(name, version, tarball, {
        env, builder, verifyBundle: () => assert.fail("failed registration cannot reach verification"),
      }), (error) => error.code === code);
    });
  });
}

test("a recovered entry cannot bypass signature or proof verification failure", async () => {
  await withRekor({}, async ({ builder }) => {
    await assert.rejects(protocolProvenance(name, version, tarball, {
      env, builder, verifyBundle: async () => { throw new Error("untrusted proof"); },
    }), /untrusted proof/);
  });
});

for (const outcome of ["success", "proof-failure", "rejected", "lost-response", "already-published", "npm-already-published", "crate-already-published", "mismatched-published", "registry-error"]) {
  test(`protocol publication preserves verified proof and handles ${outcome}`, () => {
    const failProof = outcome === "proof-failure";
    const root = fs.mkdtempSync(path.join(os.tmpdir(), "foundation-publish-proof-"));
    const bin = path.join(root, "bin");
    fs.mkdirSync(bin);
    fs.mkdirSync(path.join(root, "crates/core-api"), { recursive: true });
    fs.writeFileSync(path.join(root, "crates/core-api/Cargo.toml"), `version = "${version}"\n`);
    const mock = `#!${process.execPath}
const fs = require('node:fs'), path = require('node:path');
const tool = path.basename(process.argv[1]), args = process.argv.slice(2);
fs.appendFileSync(process.env.CALL_LOG, JSON.stringify([tool, ...args]) + '\\n');
if (tool === 'cargo' && args[0] === 'metadata') console.log(JSON.stringify({target_directory: process.env.FAKE_TARGET}));
if (tool === 'cargo' && args[0] === 'package') {
  fs.mkdirSync(path.join(process.env.FAKE_TARGET, 'package'), {recursive:true});
  fs.writeFileSync(path.join(process.env.FAKE_TARGET, 'package', 'mhome-core-api-${version}.crate'), 'prepared archive');
}
if (tool === 'npm' && args[0] === '--version') console.log('12.2.1');
if (tool === 'npm' && args[0] === 'pack') fs.writeFileSync(path.join(args[args.indexOf('--pack-destination')+1], 'protocol.tgz'), 'tarball');
if (tool === 'npm' && args[0] === 'publish' && args.includes('--dry-run') && ['already-published', 'npm-already-published'].includes(process.env.UPLOAD_OUTCOME)) {
  console.error('You cannot publish over the previously published versions');
  process.exit(1);
}
if ((tool === 'npm' && args[0] === 'view') || tool === 'curl') process.exit(1);
if (tool === 'node' && args[0] === 'scripts/protocol-provenance.mjs') {
  if (process.env.FAIL_PROOF === 'true') process.exit(1);
  fs.writeFileSync(args.at(-1), 'verified bundle');
}
if (tool === 'node' && args[0].endsWith('/upload-npm.mjs')) {
  const result = require('node:child_process').spawnSync(process.execPath, args, {stdio:'inherit', env:process.env});
  process.exit(result.status ?? 1);
}
if (tool === 'npm' && args[0] === 'publish' && !args.includes('--dry-run')) {
  if (!args.includes('--provenance-file') || args.includes('--provenance')) process.exit(2);
  if (fs.readFileSync(args[args.indexOf('--provenance-file')+1], 'utf8') !== 'verified bundle') process.exit(3);
  if (['rejected', 'lost-response'].includes(process.env.UPLOAD_OUTCOME)) {
    console.log(JSON.stringify({error:{code:process.env.UPLOAD_OUTCOME === 'rejected' ? 'E404' : 'ECONNRESET'}}));
    process.exit(1);
  }
}
`;
    for (const tool of ["cargo", "curl", "node", "npm"]) fs.writeFileSync(path.join(bin, tool), mock, { mode: 0o755 });
    fs.writeFileSync(path.join(bin, "python3"), registryPythonFixture(`
require('node:fs').appendFileSync(process.env.CALL_LOG, JSON.stringify(['python3', ...args]) + '\\n');
const outcome = process.env.UPLOAD_OUTCOME;
if (operation === 'exists') {
  if (outcome === 'registry-error' && args[3] === 'npm') process.exit(2);
  if (outcome === 'npm-already-published') process.exit(args[3] === 'npm' ? 0 : 3);
  if (outcome === 'crate-already-published') process.exit(args[3] === 'crate' ? 0 : 3);
  process.exit(['already-published', 'mismatched-published'].includes(outcome) ? 0 : 3);
}
if (operation === 'verify' && args[3] === 'npm' && outcome === 'mismatched-published') process.exit(2);
`), { mode: 0o755 });
    const callLog = path.join(root, "calls.jsonl");
    try {
      const result = spawnSync("bash", [fileURLToPath(new URL("../publish-tag.sh", import.meta.url)), `mhome-core-api-v${version}`, "--publish"], {
        cwd: root, encoding: "utf8", timeout: 15000, env: { ...process.env, PATH: `${bin}:${process.env.PATH}`, CALL_LOG: callLog, FAIL_PROOF: String(failProof), UPLOAD_OUTCOME: outcome, FAKE_TARGET: path.join(root, 'target') },
      });
      const calls = fs.readFileSync(callLog, "utf8").trim().split("\n").map(JSON.parse);
      const proof = calls.findIndex(([tool, arg]) => tool === "node" && arg === "scripts/protocol-provenance.mjs");
      const uploads = calls.map((call, i) => ({ call, i })).filter(({ call }) => ["cargo", "npm"].includes(call[0]) && call[1] === "publish" && !call.includes("--dry-run"));
      const existingOrError = ["already-published", "npm-already-published", "mismatched-published", "registry-error"].includes(outcome);
      assert.equal(proof >= 0, !existingOrError);
      assert.equal(result.status, outcome === "registry-error" || outcome === "mismatched-published" ? 2 : failProof || outcome === "rejected" ? 1 : 0, result.stderr);
      const partial = ["npm-already-published", "crate-already-published"].includes(outcome);
      assert.equal(uploads.length, partial ? 1 : failProof || existingOrError ? 0 : 2);
      if (!existingOrError) assert.ok(uploads.every(({ i }) => i > proof));
      if (partial) assert.equal(uploads[0].call[0], outcome === 'npm-already-published' ? 'cargo' : 'npm');
      const waits = calls.filter(call => call[0] === 'python3' && call[3] === 'wait' && call[4] === 'npm');
      assert.equal(waits.length, failProof || existingOrError || outcome === 'rejected' ? 0 : 1);
      if (['already-published', 'npm-already-published'].includes(outcome)) {
        assert.equal(calls.some(call => call[0] === 'npm' && call[1] === 'publish'), false);
        assert.ok(calls.some(call => call[0] === 'python3' && call[3] === 'verify' && call[4] === 'npm'));
      }
      if (outcome === 'rejected') assert.match(result.stderr, /npm rejected publication \(E404\)/);
    } finally { fs.rmSync(root, { recursive: true, force: true }); }
  });
}
