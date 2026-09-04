import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import { PROTOCOL_PACKAGES } from "../protocol-packages.mjs";
import { stageProtocolPackage } from "../stage-protocol-package.mjs";

for (const [name, config] of Object.entries(PROTOCOL_PACKAGES)) {
  test(`stages the ${name} protocol without executable runtime code`, () => {
    const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), `mhome-${name}-protocol-`));
    try {
      const staged = stageProtocolPackage(name, path.join(tempRoot, "package"));
      assert.equal(staged.packageJson.name, config.npmName);
      assert.equal(staged.packageJson.version, staged.descriptor.contractVersion);
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
      assert.deepEqual(staged.packageJson.exports["./protocol.json"], "./protocol.json");
      assert.ok(fs.existsSync(path.join(staged.stagingRoot, config.manifest)));
      for (const artifact of [
        ...Object.values(config.schemas),
        ...Object.values(config.fixtures),
      ]) {
        assert.ok(fs.existsSync(path.join(staged.stagingRoot, artifact)), artifact);
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

test("refuses to overwrite a non-empty staging directory", () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "mhome-protocol-staging-"));
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
