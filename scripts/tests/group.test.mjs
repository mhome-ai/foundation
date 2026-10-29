import assert from 'node:assert/strict';
import fs from 'node:fs';
import test from 'node:test';
import Ajv from 'ajv';
import addFormats from 'ajv-formats';
const read = p => JSON.parse(fs.readFileSync(new URL(p, import.meta.url), 'utf8'));
const schema = read('../../crates/app-facade-api/schema/group.v1.schema.json');
const corpus = read('../../crates/app-facade-api/fixtures/group.conformance.json');
test('Group definitions and mutations enforce their concrete contracts under strict AJV', () => {
  const ajv = new Ajv({strict: true, allErrors: true}); addFormats(ajv); ajv.addSchema(schema);
  for (const [kind, expected] of [['valid', true], ['invalid', false]]) {
    for (const item of corpus[kind]) {
      const validate = ajv.compile({$ref: `${schema.$id}#/$defs/${item.definition}`});
      assert.equal(validate(item.value), expected, `${item.definition}: ${JSON.stringify(validate.errors)}`);
    }
  }
});
