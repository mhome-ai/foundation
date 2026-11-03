import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { publishScriptTag } from "../publish-script-tag.mjs";
import { protocolStatement } from "../protocol-provenance.mjs";

function fixture(kind = "sdk", behavior = {}) {
  const tag = kind === "sdk" ? "meowlink-script-v0.1.0" : "meowlink-script-build-v0.1.0";
  const calls = [], manifests = new Map();
  const env = {
    GITHUB_ACTIONS: "true", RUNNER_ENVIRONMENT: "github-hosted",
    GITHUB_REF: `refs/tags/${tag}`, GITHUB_REPOSITORY: "mhome-ai/foundation",
    GITHUB_WORKFLOW_REF: `mhome-ai/foundation/.github/workflows/publish-crate.yml@refs/tags/${tag}`,
  };
  const options = { publish: true, env,
    stage(kind, directory) {
      fs.mkdirSync(directory);
      const manifest = { name: kind === "sdk" ? "@meowlink/script" : "@meowlink/script-build", version: "0.1.0" };
      manifests.set(directory, manifest);
      return manifest;
    },
    run(command, args) {
      calls.push([command, ...args]);
      if (command === "npm" && args[0] === "--version") return "12.2.1";
      if (args[0] === "pack") {
        const manifest = manifests.get(args[1]);
        const filename = `${path.basename(args[1])}.tgz`;
        fs.writeFileSync(path.join(args.at(-1), filename), "fixture");
        return JSON.stringify([{ ...manifest, filename }]);
      }
      if (command === "python3" && args[2] === "exists" && !behavior.exists) {
        throw Object.assign(new Error("registry"), { status: behavior.registryFailure ?? 3 });
      }
      if (command === "node" && behavior.proofFailure) throw new Error("proof rejected");
      if (command === "python3" && args[2] === "verify" && args[4] === "@meowlink/script" && behavior.sdkMismatch) throw new Error("SDK content mismatch");
      if (args[0] === "publish" && behavior.uploadLost) throw Object.assign(new Error("lost response"), { status: 1 });
      return "";
    },
  };
  return { tag, options, calls };
}

test("unknown tags, mismatched versions and untrusted publication fail before commands", () => {
  const { options, calls } = fixture();
  for (const tag of ["other-v0.1.0", "meowlink-script-v9.9.9", "meowlink-script-vlatest"]) {
    assert.throws(() => publishScriptTag(tag, options));
  }
  assert.throws(() => publishScriptTag("meowlink-script-v0.1.0", { ...options, env: {} }));
  assert.deepEqual(calls, []);
});

test("local verification never queries or publishes a registry", () => {
  const { tag, options, calls } = fixture();
  publishScriptTag(tag, { ...options, publish: false, env: {} });
  assert.ok(calls.some(call => call.includes("pack")));
  assert.ok(!calls.some(call => call[0] === "python3" || call.includes("publish")));
});

test("builder verifies its published SDK before generating proof or uploading", () => {
  const { tag, options, calls } = fixture("build");
  publishScriptTag(tag, options);
  const sdk = calls.findIndex(call => call[0] === "python3" && call.includes("verify") && call.includes("@meowlink/script"));
  const proof = calls.findIndex(call => call[0] === "node");
  const upload = calls.findIndex(call => call.includes("publish"));
  assert.ok(sdk >= 0 && proof > sdk && upload > proof);
  assert.ok(calls[upload].includes("--provenance-file"));
  assert.equal(calls.at(-1)[3], "verify");
  const mismatch = fixture("build", { sdkMismatch: true });
  assert.throws(() => publishScriptTag(mismatch.tag, mismatch.options), /SDK content mismatch/);
  assert.ok(!mismatch.calls.some(call => call.includes("publish")));
});

test("registry and proof errors stop upload; existing versions are verified", () => {
  for (const behavior of [{ registryFailure: 1 }, { proofFailure: true }]) {
    const { tag, options, calls } = fixture("sdk", behavior);
    assert.throws(() => publishScriptTag(tag, options));
    assert.ok(!calls.some(call => call.includes("publish")));
  }
  const { tag, options, calls } = fixture("sdk", { exists: true });
  publishScriptTag(tag, options);
  assert.ok(!calls.some(call => call.includes("publish") || call[0] === "node"));
  assert.equal(calls.at(-1)[3], "verify");
});

test("lost upload response triggers verification without another upload", () => {
  const { tag, options, calls } = fixture("sdk", { uploadLost: true });
  publishScriptTag(tag, options);
  assert.equal(calls.filter(call => call.includes("publish")).length, 1);
  assert.equal(calls.at(-2)[3], "wait");
  assert.equal(calls.at(-1)[3], "verify");
});

test("script packages use the canonical workflow's byte-bound provenance", () => {
  for (const kind of ["sdk", "build"]) {
    const { options: { env } } = fixture(kind);
    const name = kind === "sdk" ? "@meowlink/script" : "@meowlink/script-build";
    const statement = protocolStatement(name, "0.1.0", Buffer.from("packed bytes"), {
      ...env, GITHUB_SERVER_URL: "https://github.com", GITHUB_SHA: "a".repeat(40),
      GITHUB_RUN_ID: "1", GITHUB_RUN_ATTEMPT: "1",
    });
    assert.match(statement.subject[0].name, /meowlink/);
    assert.equal(statement.subject[0].digest.sha512.length, 128);
  }
});
