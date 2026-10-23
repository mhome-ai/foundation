import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import Ajv from "ajv";
import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";

import { PROTOCOL_PACKAGES } from "../protocol-packages.mjs";

const root = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../.."
);
const readJson = (file) => JSON.parse(fs.readFileSync(file, "utf8"));

function compiler(draft = "https://json-schema.org/draft/2020-12/schema") {
  const Constructor = draft === "http://json-schema.org/draft-07/schema#" ? Ajv : Ajv2020;
  const ajv = new Constructor({ allErrors: true, strict: true });
  addFormats(ajv);
  return ajv;
}

for (const [name, config] of Object.entries(PROTOCOL_PACKAGES)) {
  test(`${name} protocol schemas compile with strict consumer AJV settings`, () => {
    const compilers = new Map();
    const schemas = [...new Set(Object.values(config.schemas))].map((file) => ({
      file,
      schema: readJson(path.join(root, config.crateDir, file)),
    }));
    for (const { file, schema } of schemas) {
      if (!compilers.has(schema.$schema)) compilers.set(schema.$schema, compiler(schema.$schema));
      compilers.get(schema.$schema).addSchema(schema, file);
    }
    for (const { file, schema } of schemas) {
      assert.equal(typeof compilers.get(schema.$schema).getSchema(file), "function", file);
    }
  });
}

for (const name of ["artifactPut", "artifactResolve", "artifactUpload", "artifactImport"]) {
  test(`${name} accepts valid fixtures and rejects invalid fixtures in AJV`, () => {
    const config = PROTOCOL_PACKAGES.artifact;
    const validate = compiler().compile(
      readJson(path.join(root, config.crateDir, config.schemas[name]))
    );
    const fixtures = readJson(
      path.join(root, config.crateDir, config.fixtures[name])
    );
    for (const item of fixtures.valid) {
      assert.equal(
        validate(item.value),
        true,
        `${item.name}: ${JSON.stringify(validate.errors)}`
      );
    }
    for (const item of fixtures.invalid) {
      assert.equal(validate(item.value), false, item.name);
    }
  });
}

for (const name of ["artifactPut", "artifactResolve"]) {
  test(`${name} requires expiry only for temporary responses`, () => {
    const config = PROTOCOL_PACKAGES.artifact;
    const validate = compiler().compile(
      readJson(path.join(root, config.crateDir, config.schemas[name]))
    );
    const fixtures = readJson(
      path.join(root, config.crateDir, config.fixtures[name])
    );
    const response = fixtures.valid.find(
      ({ value }) => value.uri && value.kind
    ).value;
    const expiresAtUnixMs = 1900000000000;
    for (const identity of ["auto", "man"]) {
      for (const retention of ["std", "tmp"]) {
        const parts = response.uri.split("/");
        parts[5] = retention;
        parts[6] = identity;
        if (identity === "man") parts[7] = "Caller-A";
        const value = { ...response, identity, retention, uri: parts.join("/") };
        delete value.expiresAtUnixMs;
        assert.equal(validate(value), retention === "std");
        assert.equal(validate({ ...value, expiresAtUnixMs }), retention === "tmp");
        assert.equal(validate({ ...value, expiresAtUnixMs: 0 }), false);
        assert.equal(validate({ ...value, identity: null }), false);
        assert.equal(validate({ ...value, retention: null }), false);
        assert.equal(validate({ ...value, lifecycle: "std" }), false);
      }
    }
  });
}

test("camera settings fixtures match the shared management schema in AJV", () => {
  const config = PROTOCOL_PACKAGES.core;
  const name = "cameraPluginManagement";
  const validate = compiler().compile(
    readJson(path.join(root, config.crateDir, config.schemas[name]))
  );
  const fixtures = readJson(
    path.join(root, config.crateDir, config.fixtures[name])
  );
  for (const [section, expected] of [["valid", true], ["invalid", false]]) {
    for (const item of fixtures[section]) {
      assert.equal(
        validate(item.value),
        expected,
        `${item.name}: ${JSON.stringify(validate.errors)}`
      );
    }
  }
});

for (const name of ["artifactUploadStart", "artifactUploadReceipt", "artifactUploadComplete", "artifactUploadCancel"]) {
  test(`${name} control messages match streaming fixtures in AJV`, () => {
    const config = PROTOCOL_PACKAGES.artifact;
    const schema = readJson(path.join(root, config.crateDir, config.schemas[name]));
    const fixture = readJson(path.join(root, config.crateDir, config.fixtures[name]));
    const ajv = compiler();
    ajv.addSchema(schema);
    for (const [section, expected] of [["valid", true], ["invalid", false]]) {
      for (const item of fixture[section]) {
        const ref = item.definition === "root" ? schema.$id : `${schema.$id}#/$defs/${item.definition}`;
        const validate = ajv.getSchema(ref);
        assert.equal(validate(item.value), expected, `${item.name}: ${JSON.stringify(validate.errors)}`);
      }
    }
  });
}

test("password fields reject defaults and read-only values", () => {
  const config = PROTOCOL_PACKAGES.core;
  const schema = readJson(
    path.join(root, config.crateDir, config.schemas.interactionFlowNode)
  );
  const ajv = compiler();
  ajv.addSchema(schema);
  const validate = ajv.compile({ $ref: `${schema.$id}#/$defs/field` });
  const field = { var: "$password", valueType: "string", control: "password" };
  assert.equal(validate(field), true);
  assert.equal(validate({ ...field, readOnly: false }), true);
  assert.equal(validate({ ...field, defaultValue: "secret" }), false);
  assert.equal(
    validate({ ...field, defaultValueRef: { root: "$secret" } }),
    false
  );
  assert.equal(validate({ ...field, readOnly: true }), false);
  assert.equal(
    validate({ ...field, control: "text", defaultValue: "visible" }),
    true
  );
});

test("facade approval fields follow the selected control mode", () => {
  const config = PROTOCOL_PACKAGES.appFacade;
  const validate = compiler().compile(
    readJson(path.join(root, config.crateDir, config.schemas.call))
  );
  const call = (mode, fields = {}) => ({
    control: { mode, ...fields },
    input: {},
  });
  for (const mode of ["direct", "prepare"]) {
    assert.equal(validate(call(mode)), true);
    assert.equal(validate(call(mode, { preparedActionId: "p" })), false);
    assert.equal(validate(call(mode, { approvalToken: "t" })), false);
  }
  assert.equal(
    validate(call("commit", { preparedActionId: "p", approvalToken: "t" })),
    true
  );
  assert.equal(validate(call("commit", { preparedActionId: "p" })), false);
  assert.equal(validate(call("reject", { preparedActionId: "p" })), true);
  assert.equal(
    validate(call("reject", { preparedActionId: "p", approvalToken: "t" })),
    false
  );
});
