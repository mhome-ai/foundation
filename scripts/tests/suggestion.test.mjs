import fs from "node:fs";
import test from "node:test";
import assert from "node:assert/strict";
import Ajv from "ajv/dist/2020.js";

const root = new URL("../../", import.meta.url);
const read = path => JSON.parse(fs.readFileSync(new URL(path, root), "utf8"));

test("Suggestion receipts embed the canonical Facade result without another outcome model", () => {
  const canonical = read("crates/app-facade-api/schema/facade-response.v1.schema.json");
  const embedded = JSON.parse(JSON.stringify(canonical, (key, value) =>
    key === "$ref" && value.startsWith("#/")
      ? `#/$defs/ActionResult/${value.slice(2)}` : value
  ));
  delete embedded.$schema;
  delete embedded.$id;
  assert.deepEqual(read("crates/app-facade-api/schema/suggestion.v1.schema.json").$defs.ActionResult, embedded);
});

for (const [name, schemaPath, fixturePath, fixedType] of [
  ["Suggestion", "crates/app-facade-api/schema/suggestion.v1.schema.json", "crates/app-facade-api/fixtures/suggestion.conformance.json"],
  ["Suggestion execution", "crates/conversation-api/schema/execution/suggestion.v1.schema.json", "crates/conversation-api/fixtures/suggestion/conformance.json", "SuggestionInvocationContext"],
]) {
  test(`${name} shared fixtures conform under strict consumer AJV`, () => {
    const schema = read(schemaPath);
    const ajv = new Ajv({ strict: true, allErrors: true });
    ajv.addSchema(schema);
    // Compile every exported definition, including definitions not used by a fixture.
    for (const type of Object.keys(schema.$defs)) ajv.getSchema(`${schema.$id}#/$defs/${type}`);
    const corpus = read(fixturePath);
    for (const [group, expected] of [["valid", true], ["invalid", false]]) {
      for (const item of corpus[group]) {
        const validate = ajv.getSchema(`${schema.$id}#/$defs/${fixedType ?? item.type}`);
        assert.equal(validate(item.value), expected, `${item.name}: ${JSON.stringify(validate.errors)}`);
      }
    }
  });
}
