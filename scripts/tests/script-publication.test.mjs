import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { publishScriptTag } from "../publish-script-tag.mjs";
import { protocolStatement } from "../protocol-provenance.mjs";

const packageInfo = Object.fromEntries([["sdk", "script-sdk"], ["build", "script-build"]].map(([kind, directory]) =>
  [kind, JSON.parse(fs.readFileSync(new URL(`../../packages/${directory}/package.json`, import.meta.url), "utf8"))]));

function fixture(kind = "sdk", behavior = {}) {
  const tag = `${kind === "sdk" ? "mhome-script-v" : "mhome-script-build-v"}${packageInfo[kind].version}`;
  const calls = [], manifests = new Map();
  const env = {
    GITHUB_ACTIONS: "true", RUNNER_ENVIRONMENT: "github-hosted",
    GITHUB_REF: `refs/tags/${tag}`, GITHUB_REPOSITORY: "mhome-ai/foundation",
    GITHUB_WORKFLOW_REF: `mhome-ai/foundation/.github/workflows/publish-crate.yml@refs/tags/${tag}`,
  };
  const options = { publish: true, env,
    stage(kind, directory) {
      fs.mkdirSync(directory);
      const { name, version } = packageInfo[kind];
      const manifest = { name, version };
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
      if (command === "python3" && args[2] === "verify" && args[4] === "@mhome/script" && behavior.sdkMismatch) throw new Error("SDK content mismatch");
      if (args[0] === "publish" && behavior.uploadLost) throw Object.assign(new Error("lost response"), { status: 1 });
      if (args[0] === "publish" && behavior.uploadRejected) throw Object.assign(new Error("rejected"), {
        status: 1, stdout: JSON.stringify({ error: { code: behavior.uploadRejected } }),
      });
      return "";
    },
  };
  return { tag, options, calls };
}

test("unknown tags, mismatched versions and untrusted publication fail before commands", () => {
  const { tag: trustedTag, options, calls } = fixture();
  for (const tag of ["other-v0.1.0", "mhome-script-v9.9.9", "mhome-script-vlatest"]) {
    assert.throws(() => publishScriptTag(tag, options));
  }
  assert.throws(() => publishScriptTag(trustedTag, { ...options, env: {} }));
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
  const sdk = calls.findIndex(call => call[0] === "python3" && call.includes("verify") && call.includes("@mhome/script"));
  const proof = calls.findIndex(call => call[0] === "node");
  const upload = calls.findIndex(call => call.includes("publish"));
  assert.ok(sdk >= 0 && proof > sdk && upload > proof);
  assert.equal(calls[sdk][6], packageInfo.sdk.version, "builder verifies the SDK release, not its own version");
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

test("explicit npm authorization rejection stops without waiting for an impossible upload", () => {
  for (const uploadRejected of ["E401", "E402", "E403", "E404", "ENEEDAUTH", "EOTP"]) {
    const { tag, options, calls } = fixture("sdk", { uploadRejected });
    assert.throws(() => publishScriptTag(tag, options), new RegExp(`npm rejected publication \\(${uploadRejected}\\)`));
    assert.equal(calls.filter(call => call.includes("publish")).length, 1);
    assert.ok(calls.find(call => call.includes("publish")).includes("--json"));
    assert.ok(!calls.some(call => call[0] === "python3" && ["wait", "verify"].includes(call[3])));
  }
});

test("script packages use the canonical workflow's byte-bound provenance", () => {
  for (const kind of ["sdk", "build"]) {
    const { options: { env } } = fixture(kind);
    const name = kind === "sdk" ? "@mhome/script" : "@mhome/script-build";
    const statement = protocolStatement(name, packageInfo[kind].version, Buffer.from("packed bytes"), {
      ...env, GITHUB_SERVER_URL: "https://github.com", GITHUB_SHA: "a".repeat(40),
      GITHUB_RUN_ID: "1", GITHUB_RUN_ATTEMPT: "1",
    });
    assert.match(statement.subject[0].name, /mhome/);
    assert.equal(statement.subject[0].digest.sha512.length, 128);
  }
});

test("read-only registry verification compares packed SDK and builder without publication credentials", () => {
  for (const kind of ["sdk", "build"]) {
    const { tag, options, calls } = fixture(kind);
    publishScriptTag(tag, { ...options, publish: false, verifyPublished: true, env: {} });
    const registry = calls.filter(call => call[0] === "python3");
    assert.deepEqual(registry.map(call => [call[3], call[5]]), kind === "sdk"
      ? [["verify", "@mhome/script"]]
      : [["verify", "@mhome/script"], ["verify", "@mhome/script-build"]]);
    assert.ok(!calls.some(call => call[0] === "node" || call.includes("publish") || call.includes("--version")));
  }
});

test("read-only verification rejects changed package contents and incompatible modes", () => {
  const { tag, options, calls } = fixture("sdk", { sdkMismatch: true });
  assert.throws(() => publishScriptTag(tag, { ...options, publish: false, verifyPublished: true, env: {} }), /SDK content mismatch/);
  assert.ok(!calls.some(call => call.includes("publish")));
  const conflict = fixture();
  assert.throws(() => publishScriptTag(conflict.tag, { ...conflict.options, verifyPublished: true }), /mutually exclusive/);
  assert.deepEqual(conflict.calls, []);
});
