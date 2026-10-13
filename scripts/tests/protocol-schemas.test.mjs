import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";

import { PROTOCOL_PACKAGES } from "../protocol-packages.mjs";

const root = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../.."
);
const readJson = (file) => JSON.parse(fs.readFileSync(file, "utf8"));

function compiler() {
  const ajv = new Ajv2020({ allErrors: true, strict: true });
  addFormats(ajv);
  return ajv;
}

for (const [name, config] of Object.entries(PROTOCOL_PACKAGES)) {
  test(`${name} protocol schemas compile with strict consumer AJV settings`, () => {
    const ajv = compiler();
    const schemas = [...new Set(Object.values(config.schemas))].map((file) => ({
      file,
      schema: readJson(path.join(root, config.crateDir, file)),
    }));
    for (const { file, schema } of schemas) ajv.addSchema(schema, file);
    for (const { file } of schemas) {
      assert.equal(typeof ajv.getSchema(file), "function", file);
    }
  });
}

for (const name of ["artifactPut", "artifactResolve", "artifactUpload"]) {
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
    assert.equal(
      validate({ ...response, temporary: true, expiresAtUnixMs }),
      true
    );
    assert.equal(validate({ ...response, temporary: false }), true);
    assert.equal(validate({ ...response, temporary: true }), false);
    assert.equal(validate({ ...response, expiresAtUnixMs }), false);
    assert.equal(
      validate({ ...response, temporary: false, expiresAtUnixMs }),
      false
    );
    assert.equal(
      validate({ ...response, temporary: true, expiresAtUnixMs: 0 }),
      false
    );
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
