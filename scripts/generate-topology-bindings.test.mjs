import {test} from 'node:test';
import assert from 'node:assert/strict';
import {mkdtempSync, appendFileSync, rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {spawnSync} from 'node:child_process';

test('bindings are reproducible and drift is rejected', () => {
  const destination = mkdtempSync(join(tmpdir(), 'topology-bindings-test-'));
  try {
    const script = fileURLToPath(new URL('./generate-topology-bindings.mjs', import.meta.url));
    const args = [script, '--meowcore', destination, '--baycat', destination, '--lion-service', destination];
    const run = extra => spawnSync(process.execPath, [...args, ...extra], {encoding:'utf8'});
    assert.equal(run([]).status, 0);
    assert.equal(run(['--check']).status, 0);
    appendFileSync(join(destination, 'src/application/device_topology/camera_contract.generated.rs'), '// drift\n');
    const drift = run(['--check']);
    assert.notEqual(drift.status, 0);
    assert.match(drift.stderr, /Stale generated binding/);
  } finally {
    rmSync(destination, {recursive:true, force:true});
  }
});
