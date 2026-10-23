import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import addFormats from "ajv-formats";
import Ajv from "ajv";
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../crates/app-facade-api");
const read = (file) => JSON.parse(fs.readFileSync(path.join(root, file), "utf8"));
const wire = read("schema/card-skins.v1.schema.json");
const ajv = new Ajv({ strict: false });
addFormats(ajv);
ajv.addSchema(read("schema/card-skin-package.v1.schema.json"));
ajv.addSchema(wire);
const valid = (definition, value) => ajv.validate({ $ref: `${wire.$id}#/definitions/${definition}` }, value);
test("Space edits require an explicit revision and cannot supply release data", () => {
  const input = { expectedRevision: null, packageId: "example.evening", version: "1.0.0" };
  assert.ok(valid("install", input));
  for (const extra of ["releaseId", "manifest", "baseUrl", "spaceId", "tenantId", "authorUserId"])
    assert.equal(valid("install", { ...input, [extra]: "forged" }), false);
  assert.equal(valid("install", { packageId: input.packageId, version: input.version }), false);
  assert.equal(valid("install", { ...input, expectedRevision: false }), false);
  assert.ok(valid("overrideSet", { expectedRevision: "r1", patternId: "basic.device.onoff_brightness_color", installationId: "install", rendererId: "evening-light", config: {} }));
});
test("card skin routes share one policy and schema inventory", () => {
  const requests = read("manifest/app-facade.v1.json").domains.cardSkins.requests;
  const policies = read("manifest/authorization.v1.json").operations;
  const routing = read("manifest/routing.v1.json").rules;
  assert.ok(routing.cloudPrefixes.includes("/app/card-skins/catalog/"));
  assert.equal(Object.keys(requests).length, 13);
  for (const [target, reference] of Object.entries(requests)) {
    assert.ok(wire.definitions[reference.split("/").at(-1)]);
    assert.ok(policies[target], target);
  }
});
test("the package protocol bounds binary uploads without changing skin interfaces", () => {
  const schema = read("schema/card-skin-package.v1.schema.json");
  assert.equal(schema.properties.apiVersion.const, 1);
  assert.equal(schema.properties.files.maxItems, 128);
  assert.equal(schema.definitions.file.properties.sizeBytes.maximum, 8 * 1024 * 1024);
  assert.ok(ajv.validateSchema(schema));
});

test("persisted Space selection and immutable release match the public response schema", () => {
  const fixture = read("fixtures/card-skins/space.json");
  assert.ok(valid("selectionView", { revision: "revision", selection: fixture.selection }), JSON.stringify(ajv.errors));
  assert.equal(valid("selectionView", { revision: null, selection: { installations: [], overrides: [], releases: [] }, tenantId: "forged" }), false);
});
