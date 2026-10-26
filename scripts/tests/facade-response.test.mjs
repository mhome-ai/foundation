import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";

import { PROTOCOL_PACKAGES } from "../protocol-packages.mjs";
import { stageProtocolPackage } from "../stage-protocol-package.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const read = (file) => JSON.parse(fs.readFileSync(file, "utf8"));
const app = PROTOCOL_PACKAGES.appFacade;
const conversation = PROTOCOL_PACKAGES.conversation;
const responseSchema = read(path.join(root, app.crateDir, app.schemas.response));
const executionSchema = read(path.join(root, conversation.crateDir, conversation.schemas.execution));
const fixtures = read(path.join(root, conversation.crateDir, conversation.fixtures.facadeResponse));
const preparedEnvelope = read(path.join(root, conversation.crateDir, "fixtures/execution/app-facade-prepared.v1.json"));

function compile(schema) {
  const ajv = new Ajv2020({ allErrors: true, strict: true });
  addFormats(ajv);
  return ajv.compile(schema);
}

function checkCases(validate, wrap = (value) => value) {
  for (const [section, expected] of [["valid", true], ["invalid", false]]) {
    for (const { name, value } of fixtures[section]) {
      assert.equal(validate(wrap(value)), expected, `${name}: ${JSON.stringify(validate.errors)}`);
    }
  }
}

test("standalone and embedded Facade response definitions stay identical", () => {
  const embedded = JSON.parse(JSON.stringify(responseSchema, (key, value) =>
    key === "$ref" && value.startsWith("#/")
      ? `#/$defs/facade_response/${value.slice(2)}`
      : value
  ));
  delete embedded.$schema;
  assert.deepEqual(executionSchema.$defs.facade_response, embedded);
});

test("Facade responses enforce all four outcomes with strict consumer AJV", () => {
  checkCases(compile(responseSchema));
});

test("execution envelopes enforce the same Facade response contract", () => {
  checkCases(compile(executionSchema), (value) => ({
    ...preparedEnvelope,
    payload: {
      family: "app_facade_response",
      payload: { type: "result", result: { value } },
    },
  }));
});

test("staged protocol packages expose usable response schemas and conformance cases", () => {
  const temp = fs.mkdtempSync(path.join(os.tmpdir(), "mhome-facade-response-"));
  try {
    const stagedApp = stageProtocolPackage("appFacade", path.join(temp, "app"));
    const stagedConversation = stageProtocolPackage("conversation", path.join(temp, "conversation"));
    const appDescriptor = read(path.join(stagedApp.stagingRoot, "protocol.json"));
    const conversationDescriptor = read(path.join(stagedConversation.stagingRoot, "protocol.json"));
    assert.equal(appDescriptor.schemas.response, "schema/facade-response.v1.schema.json");
    const packagedCases = read(path.join(stagedConversation.stagingRoot, conversationDescriptor.fixtures.facadeResponse));
    assert.deepEqual(packagedCases, fixtures);
    checkCases(compile(read(path.join(stagedApp.stagingRoot, appDescriptor.schemas.response))));
    checkCases(compile(read(path.join(stagedConversation.stagingRoot, conversationDescriptor.schemas.execution))), (value) => ({
      ...preparedEnvelope,
      payload: { family: "app_facade_response", payload: { type: "result", result: { value } } },
    }));
  } finally {
    fs.rmSync(temp, { recursive: true, force: true });
  }
});
