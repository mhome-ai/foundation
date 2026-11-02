import fs from 'node:fs';
import test from 'node:test';
import assert from 'node:assert/strict';
import Ajv from 'ajv/dist/2020.js';

const root = new URL('../../crates/app-facade-api/', import.meta.url);
const schema = JSON.parse(fs.readFileSync(new URL('schema/now-attention.v1.schema.json', root)));
const ajv = new Ajv({strict: true});
ajv.addSchema(schema);
const validate = name => ajv.getSchema(`${schema.$id}#/$defs/${name}`);

test('Attention has a strict viewer-free input and preserves source failure and uncertainty', () => {
  assert.ok(validate('Input')({}));
  assert.equal(validate('Input')({userId: 'other'}), false);
  const item = {id: 'device:water:episode', reasonCode: 'device.waterDetected', title: 'Water detected', summary: '',
    source: {kind: 'device', id: 'water', capabilityId: 'state.water'}, score: 74,
    time: {mode: 'persistent'}, sourceUpdatedAt: 1000, verificationRequired: true};
  const output = {items: [item], evaluatedAt: 2000, nextRefreshAt: 32000, policyVersion: 'attention.v1',
    sourceErrors: [{source: 'dialog', code: 'UNAVAILABLE'}]};
  assert.ok(validate('Output')(output), JSON.stringify(validate('Output').errors));
  assert.equal(validate('Output')({...output, items: [{...item, score: 101}]}), false);
  assert.equal(validate('Output')({...output, items: [{...item, verificationRequired: undefined}]}), false);
  assert.equal(validate('Context')({reasonCode: 'dialog.pending', association: {kind: 'personal'}, time: {mode: 'persistent'}}), false);
  assert.equal(validate('Time')({mode: 'decay', occurredAt: 1, halfLifeMs: 0}), false);
});
