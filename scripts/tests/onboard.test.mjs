import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";
import Ajv2020 from "ajv/dist/2020.js";

const root = new URL("../../crates/onboard-api/", import.meta.url);
const schema = JSON.parse(fs.readFileSync(new URL("schema/onboard.v1.schema.json", root)));
const corpus = JSON.parse(fs.readFileSync(new URL("fixtures/conformance.json", root)));

test("onboarding validates the canonical corpus without business Integration fields", () => {
  const ajv = new Ajv2020({ strict: false, allErrors: true });
  for (const fixture of corpus.cases) {
    const validate = ajv.compile({ ...schema, $id: undefined, $ref: `#/$defs/${fixture.kind}` });
    assert.equal(validate(fixture.value), fixture.valid,
      `${fixture.name}: ${JSON.stringify(validate.errors)}`);
  }
});
