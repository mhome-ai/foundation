import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";

import { PROTOCOL_PACKAGES } from "../protocol-packages.mjs";
import { stageProtocolPackage } from "../stage-protocol-package.mjs";

function manifestFileReferences(value, references = new Set()) {
  if (typeof value === "string") {
    if (/^(manifest|schema|fixtures|contract)\//.test(value)) {
      references.add(value.split("#")[0]);
    }
  } else if (Array.isArray(value)) {
    value.forEach((item) => manifestFileReferences(item, references));
  } else if (value && typeof value === "object") {
    Object.values(value).forEach((item) => manifestFileReferences(item, references));
  }
  return references;
}

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

  test(`packs every file referenced by the ${name} protocol manifests`, () => {
    const tempRoot = fs.mkdtempSync(
      path.join(os.tmpdir(), `mhome-${name}-protocol-pack-`)
    );
    try {
      const staged = stageProtocolPackage(name, path.join(tempRoot, "package"));
      const packResult = JSON.parse(execFileSync("npm", [
        "pack", staged.stagingRoot, "--json", "--offline", "--ignore-scripts",
        "--pack-destination", tempRoot,
      ], {
        cwd: tempRoot,
        encoding: "utf8",
        env: { ...process.env, npm_config_cache: path.join(tempRoot, "npm-cache") },
      }));
      // npm 12 keys JSON results by package name; npm 11 returns an array.
      const [packed] = Array.isArray(packResult) ? packResult : Object.values(packResult);
      assert.equal(packed.name, config.npmName);
      assert.ok(fs.statSync(path.join(tempRoot, packed.filename)).isFile());
      const packedFiles = new Set(packed.files.map((file) => file.path));
      if (name === "appFacade") {
        // Cloud response presentation loads this resource directly, not through a $ref.
        const timesPath = "manifest/presentation-times.v1.json";
        assert.ok(packedFiles.has(timesPath), `${timesPath} missing from npm package`);
        assert.deepEqual(
          JSON.parse(execFileSync("tar", ["-xOf", path.join(tempRoot, packed.filename), `package/${timesPath}`], { encoding: "utf8" })),
          JSON.parse(fs.readFileSync(new URL(`../../${config.crateDir}/${timesPath}`, import.meta.url), "utf8"))
        );
      }
      const manifests = fs.readdirSync(path.join(staged.stagingRoot, "manifest"), {
        recursive: true,
      }).filter((file) => file.endsWith(".json"));
      for (const file of manifests) {
        const relativePath = `manifest/${file}`;
        assert.ok(packedFiles.has(relativePath), `${relativePath} missing from npm package`);
        const manifest = JSON.parse(fs.readFileSync(
          path.join(staged.stagingRoot, relativePath), "utf8"
        ));
        const missing = [...manifestFileReferences(manifest)]
          .filter((reference) => !packedFiles.has(reference));
        assert.deepEqual(missing, [], `${relativePath} references unpackaged files`);
      }
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
