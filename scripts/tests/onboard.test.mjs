import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";
import Ajv2020 from "ajv/dist/2020.js";

const root = new URL("../../crates/onboard-api/", import.meta.url);
const schema = JSON.parse(fs.readFileSync(new URL("schema/onboard.v1.schema.json", root)));
const cases = ["fixtures/conformance.json", "fixtures/authority.conformance.json"]
  .flatMap(file => JSON.parse(fs.readFileSync(new URL(file, root))).cases);

test("onboarding validates the canonical corpus without business Integration fields", () => {
  const ajv = new Ajv2020({ strict: false, allErrors: true });
  for (const fixture of cases) {
    const validate = ajv.compile({ ...schema, $id: undefined, $ref: `#/$defs/${fixture.kind}` });
    assert.equal(validate(fixture.value), fixture.valid,
      `${fixture.name}: ${JSON.stringify(validate.errors)}`);
  }
});
