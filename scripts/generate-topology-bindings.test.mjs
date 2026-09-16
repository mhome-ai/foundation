import {test} from 'node:test';
import assert from 'node:assert/strict';
import {mkdtempSync, appendFileSync, readFileSync, writeFileSync, rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {spawnSync} from 'node:child_process';

test('bindings are reproducible and drift is rejected', () => {
  const destination = mkdtempSync(join(tmpdir(), 'topology-bindings-test-'));
  try {
    const script = fileURLToPath(new URL('./generate-topology-bindings.mjs', import.meta.url));
    const args = [script, '--meowcore', destination, '--baycat', destination, '--lion-service', destination, '--pallas', destination];
    const run = extra => spawnSync(process.execPath, [...args, ...extra], {encoding:'utf8'});
    assert.equal(run([]).status, 0);
    assert.equal(run(['--check']).status, 0);
    for (const target of [
      'src/application/device_topology/camera_contract.generated.rs',
      'nodes/matter/src/device_sources_contract.generated.rs',
      'src/application/device_topology/contract.generated.rs',
      'src/application/device_topology/graph_rules.generated.rs',
      'src/test/resources/topology/device-topology.v1.schema.json',
      'script/protocol/fixtures/device-topology.sources.json',
    ]) {
      const file=join(destination,target), original=readFileSync(file,'utf8');
      appendFileSync(file, '// drift\n');
      const drift = run(['--check']);
      assert.notEqual(drift.status, 0, target);
      assert.match(drift.stderr, /Stale generated binding/);
      writeFileSync(file,original);
    }

  } finally {
    rmSync(destination, {recursive:true, force:true});
  }
});
