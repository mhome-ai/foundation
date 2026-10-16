import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";

import { PROTOCOL_PACKAGES } from "../protocol-packages.mjs";
import { stageProtocolPackage } from "../stage-protocol-package.mjs";

for (const [name, config] of Object.entries(PROTOCOL_PACKAGES)) {
  test(`stages the ${name} protocol without executable runtime code`, () => {
    const tempRoot = fs.mkdtempSync(
      path.join(os.tmpdir(), `mhome-${name}-protocol-`)
    );
    try {
      const staged = stageProtocolPackage(name, path.join(tempRoot, "package"));
      assert.equal(staged.packageJson.name, config.npmName);
      assert.equal(
        staged.packageJson.version,
        staged.descriptor.contractVersion
      );
      assert.equal(staged.packageJson.license, "MIT");
      assert.deepEqual(staged.packageJson.repository, {
        type: "git",
        url: "git+https://github.com/mhome-ai/foundation.git",
        directory: config.crateDir,
      });
      assert.deepEqual(staged.packageJson.publishConfig, {
        access: "public",
        provenance: true,
      });
      assert.equal(staged.packageJson.main, undefined);
      assert.equal(staged.packageJson.module, undefined);
      assert.equal(staged.packageJson.browser, undefined);
      assert.deepEqual(
        staged.packageJson.exports["./protocol.json"],
        "./protocol.json"
      );
      assert.ok(fs.existsSync(path.join(staged.stagingRoot, config.manifest)));
      for (const artifact of [
        ...(config.additionalFiles || []),
        ...Object.values(config.schemas),
        ...Object.values(config.fixtures),
      ]) {
        assert.ok(
          fs.existsSync(path.join(staged.stagingRoot, artifact)),
          artifact
        );
        if (artifact.startsWith("manifest/") && artifact.endsWith(".json")) {
          const manifest = JSON.parse(
            fs.readFileSync(path.join(staged.stagingRoot, artifact), "utf8")
          );
          if (manifest.contractVersion !== undefined) {
            assert.equal(
              manifest.contractVersion,
              staged.packageJson.version,
              artifact
            );
          }
        }
      }
      const executableFiles = fs
        .readdirSync(staged.stagingRoot, { recursive: true })
        .filter((entry) => /\.(?:c?js|mjs|ts|tsx|jsx)$/.test(String(entry)));
      assert.deepEqual(executableFiles, []);
    } finally {
      fs.rmSync(tempRoot, { recursive: true, force: true });
    }
  });
}

test("stages a usable Pod Talk schema with its conversation fixtures", () => {
  const tempRoot = fs.mkdtempSync(
    path.join(os.tmpdir(), "mhome-conversation-talk-protocol-")
  );
  try {
    const staged = stageProtocolPackage(
      "conversation",
      path.join(tempRoot, "package")
    );
    const descriptor = JSON.parse(
      fs.readFileSync(path.join(staged.stagingRoot, "protocol.json"), "utf8")
    );
    assert.equal(
      typeof descriptor.schemas.podTalk,
      "string",
      "the published protocol must identify the canonical Pod Talk schema"
    );
    const schema = JSON.parse(
      fs.readFileSync(
        path.join(staged.stagingRoot, descriptor.schemas.podTalk),
        "utf8"
      )
    );
    const ajv = new Ajv2020({ allErrors: true, strict: true });
    addFormats(ajv);
    const validate = ajv.compile(schema);
    const fixturesRoot = path.join(staged.stagingRoot, "fixtures/talk");
    const fixtures = fs
      .readdirSync(fixturesRoot)
      .filter((file) => file.endsWith(".json"))
      .sort();
    assert.ok(fixtures.length > 0, "Pod Talk fixtures must be published");
    for (const file of fixtures) {
      const fixture = JSON.parse(
        fs.readFileSync(path.join(fixturesRoot, file), "utf8")
      );
      assert.equal(
        validate(fixture),
        true,
        `${file}: ${JSON.stringify(validate.errors)}`
      );
      assert.equal(validate({ ...fixture, unexpected: true }), false, file);
    }
  } finally {
    fs.rmSync(tempRoot, { recursive: true, force: true });
  }
});

test("refuses to overwrite a non-empty staging directory", () => {
  const tempRoot = fs.mkdtempSync(
    path.join(os.tmpdir(), "mhome-protocol-staging-")
  );
  try {
    fs.writeFileSync(path.join(tempRoot, "keep.txt"), "do not overwrite\n");
    assert.throws(
      () => stageProtocolPackage("appFacade", tempRoot),
      /Refusing to overwrite non-empty staging directory/
    );
    assert.equal(
      fs.readFileSync(path.join(tempRoot, "keep.txt"), "utf8"),
      "do not overwrite\n"
    );
  } finally {
    fs.rmSync(tempRoot, { recursive: true, force: true });
  }
});
