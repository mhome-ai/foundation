import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync} from 'node:fs';
import {createServer} from 'node:http';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import test from 'node:test';
import {dependencyRequirements, indexUrl, waitForCrates} from '../wait-for-crates.mjs';

const name = 'mhome-core-api';
const version = '1.49.0';
const requirement = {name, version};
const entry = (overrides = {}) => ({name, vers: version, yanked: false, cksum: 'a'.repeat(64), ...overrides});
const response = (entries, status = 200) => new Response(entries.map((item) => JSON.stringify(item)).join('\n'), {status});
const fixture = (fetchImpl, timeoutMs = 30) => {
  let time = 0;
  const messages = [];
  return {
    messages,
    options: {fetchImpl, timeoutMs, intervalMs: 10, now: () => time, delay: async (ms) => { time += ms; }, log: (message) => messages.push(message)},
    elapsed: () => time,
  };
};

test('Cargo index paths follow the registry layout', () => {
  for (const [crate, suffix] of [['A', '1/a'], ['ab', '2/ab'], ['Abc', '3/a/abc'], [name, 'mh/om/mhome-core-api']]) {
    assert.equal(indexUrl(crate), `https://index.crates.io/${suffix}`);
  }
  assert.throws(() => indexUrl('../bad'), /Invalid crate/);
});

test('upload API success cannot release a consumer before its exact index version appears', async () => {
  const urls = [];
  let indexCalls = 0;
  const state = fixture(async (url, options) => {
    urls.push(url);
    if (url.startsWith('https://crates.io/api/')) return new Response('{}', {status: 200});
    assert.equal(url, indexUrl(name));
    assert.equal(options.headers['Cache-Control'], 'no-cache');
    indexCalls += 1;
    return response([entry({vers: indexCalls < 3 ? '1.48.0' : version})]);
  });
  await waitForCrates([requirement], state.options);
  assert.equal(indexCalls, 3);
  assert.equal(state.elapsed(), 20);
  assert.equal(urls.some((url) => url.includes('/api/')), false);
  assert.match(state.messages.at(-1), /Cargo index ready/);
});

test('an already available version returns immediately and duplicate requirements are queried once', async () => {
  let calls = 0;
  const state = fixture(async () => { calls += 1; return response([entry({v: 2})]); });
  await waitForCrates([requirement, requirement], state.options);
  assert.equal(calls, 1);
  assert.equal(state.elapsed(), 0);
});

test('missing exact versions time out instead of accepting a newer or similar version', async () => {
  const state = fixture(async () => response([entry({vers: '1.49.1'}), entry({vers: '1.49.0-beta.1'})]));
  await assert.rejects(waitForCrates([requirement], state.options), /timed out.*mhome-core-api@1.49.0/);
  assert.equal(state.elapsed(), 30);
});

test('missing index, throttling, server errors and network failures have a bounded retry', async () => {
  for (const status of [404, 408, 429, 503, 'network']) {
    let calls = 0;
    const state = fixture(async () => {
      calls += 1;
      if (calls > 1) return response([entry()]);
      if (status === 'network') throw new TypeError('network unavailable');
      return response([], status);
    });
    await waitForCrates([requirement], state.options);
    assert.equal(calls, 2);
    assert.equal(state.elapsed(), 10);
  }
  const state = fixture(async () => { throw new Error('offline'); });
  await assert.rejects(waitForCrates([requirement], state.options), /timed out.*offline/);
  assert.equal(state.elapsed(), 30);
});

test('yanked, malformed, unsupported and forbidden responses fail without retrying', async () => {
  for (const [makeResponse, message] of [
    [() => response([entry({yanked: true})]), /yanked/],
    [() => response([entry({yanked: undefined})]), /invalid or unsupported/],
    [() => response([entry({cksum: 'bad'})]), /invalid or unsupported/],
    [() => response([entry({v: 3})]), /invalid or unsupported/],
    [() => response([entry({name: 'other-crate'})]), /unexpected package/],
    [() => new Response('{broken'), /invalid Cargo index JSON/],
    [() => response([], 403), /HTTP 403/],
  ]) {
    let calls = 0;
    const state = fixture(async () => { calls += 1; return makeResponse(); });
    await assert.rejects(waitForCrates([requirement], state.options), message);
    assert.equal(calls, 1);
    assert.equal(state.elapsed(), 0);
  }
});

test('multiple dependencies share one deadline and ready dependencies are not polled again', async () => {
  const calls = [];
  const state = fixture(async (url) => {
    calls.push(url);
    return url === indexUrl(name) ? response([entry()]) : response([]);
  });
  await assert.rejects(waitForCrates([requirement, {name: 'other-crate', version: '2.0.0'}], state.options), /timed out.*other-crate@2.0.0/);
  assert.equal(calls.filter((url) => url === indexUrl(name)).length, 1);
  assert.equal(state.elapsed(), 30);
});

test('the deadline also bounds a server that sends headers but stalls the response body', async () => {
  const server = createServer((_req, res) => { res.writeHead(200); res.flushHeaders(); });
  await new Promise((done) => server.listen(0, '127.0.0.1', done));
  try {
    const url = `http://127.0.0.1:${server.address().port}`;
    const start = performance.now();
    await assert.rejects(waitForCrates([requirement], {
      timeoutMs: 100, intervalMs: 10, log: () => {},
      fetchImpl: (_indexUrl, options) => fetch(url, options),
    }), /timed out/);
    assert.ok(performance.now() - start < 3000);
  } finally {
    server.closeAllConnections();
    await new Promise((done) => server.close(done));
  }
});

test('dependency selection uses Cargo package names and declared pins across dependency kinds', () => {
  const metadata = {
    workspace_members: ['consumer-id', 'core-id'],
    packages: [
      {id: 'consumer-id', name: 'consumer', dependencies: [
        {name, rename: 'core-api', req: '=1.49.0', path: '/core', registry: null},
        {name, req: '=1.48.0', path: '/core', registry: null, kind: 'build', target: 'cfg(windows)'},
        {name: 'serde', req: '^1', path: null},
      ]},
      {id: 'core-id', name, version: '1.50.0', dependencies: []},
    ],
  };
  assert.deepEqual(dependencyRequirements(metadata, 'consumer'), [requirement, {name, version: '1.48.0'}]);
  assert.deepEqual(dependencyRequirements(metadata, name), []);
  assert.throws(() => dependencyRequirements(metadata, 'missing'), /not found/);
  metadata.packages[0].dependencies[0].req = '^1.49';
  assert.throws(() => dependencyRequirements(metadata, 'consumer'), /unsupported release dependency/);
});

function publishFixture({dependencyFailure = false, readinessFailure = false, cargoFailure = '', published = false, dryRun = false} = {}) {
  const root = mkdtempSync(join(tmpdir(), 'foundation-publish-index-'));
  try {
    const bin = join(root, 'bin');
    mkdirSync(bin);
    mkdirSync(join(root, 'crates/os-permissions'), {recursive: true});
    writeFileSync(join(root, 'crates/os-permissions/Cargo.toml'), 'version = "1.4.30"\n');
    const audit = join(root, 'commands');
    const executable = (name, body) => writeFileSync(join(bin, name), `#!/usr/bin/env bash\nset -eu\n${body}\n`, {mode: 0o755});
    executable('node', `
printf 'node %s\\n' "$*" >> "$AUDIT"
if [[ "$2" == --dependencies-of ]]; then exit "$DEPENDENCY_STATUS"; fi
exit "$READINESS_STATUS"`);
    executable('cargo', `
printf 'cargo %s\\n' "$*" >> "$AUDIT"
if [[ "$1" == "$CARGO_FAILURE" ]]; then exit 101; fi
if [[ "$1" == publish && "$*" != *--dry-run* ]]; then
  echo 'warning: timed out waiting for registry index' >&2
fi`);
    executable('curl', 'exit "$UPLOAD_API_STATUS"');
    const args = [fileURLToPath(new URL('../publish-tag.sh', import.meta.url)), 'mhome-os-permissions-v1.4.30'];
    if (!dryRun) args.push('--publish');
    const result = spawnSync('bash', args, {
      cwd: root, encoding: 'utf8', env: {
        ...process.env, PATH: `${bin}:${process.env.PATH}`, AUDIT: audit,
        DEPENDENCY_STATUS: dependencyFailure ? '1' : '0', READINESS_STATUS: readinessFailure ? '1' : '0',
        CARGO_FAILURE: cargoFailure, UPLOAD_API_STATUS: published ? '0' : '22',
      },
    });
    return {...result, commands: readFileSync(audit, 'utf8').trim().split('\n')};
  } finally {
    rmSync(root, {recursive: true, force: true});
  }
}

test('publishing waits for dependencies before Cargo and checks its own index after upload', () => {
  const result = publishFixture();
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.commands[0], /wait-for-crates.mjs --dependencies-of mhome-os-permissions$/);
  assert.match(result.commands.at(-1), /wait-for-crates.mjs mhome-os-permissions 1.4.30$/);
  assert.equal(result.commands.filter((line) => line.startsWith('cargo publish') && !line.includes('--dry-run')).length, 1);
});

test('unavailable dependency blocks packaging and upload', () => {
  const result = publishFixture({dependencyFailure: true});
  assert.equal(result.status, 1);
  assert.equal(result.commands.length, 1);
});

test('Cargo success with an index timeout cannot produce a green release job', () => {
  const result = publishFixture({readinessFailure: true});
  assert.equal(result.status, 1);
  assert.match(result.stderr, /timed out waiting/);
  assert.equal(result.commands.filter((line) => line.startsWith('cargo publish') && !line.includes('--dry-run')).length, 1);
});

test('an already uploaded package skips uploading but must still pass index readiness', () => {
  const result = publishFixture({published: true, readinessFailure: true});
  assert.equal(result.status, 1);
  assert.equal(result.commands.some((line) => line.startsWith('cargo publish') && !line.includes('--dry-run')), false);
  assert.match(result.commands.at(-1), /mhome-os-permissions 1.4.30$/);
});

test('deterministic Cargo failures are not retried or treated as propagation delays', () => {
  const result = publishFixture({cargoFailure: 'clippy'});
  assert.equal(result.status, 101);
  assert.equal(result.commands.filter((line) => line.startsWith('cargo clippy')).length, 1);
  assert.equal(result.commands.some((line) => line.startsWith('cargo publish')), false);
});

test('verification-only publication checks dependencies without uploading or waiting for its own unpublished version', () => {
  const result = publishFixture({dryRun: true, readinessFailure: true});
  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.commands.filter((line) => line.startsWith('node ')).length, 1);
  assert.equal(result.commands.some((line) => line.startsWith('cargo publish') && !line.includes('--dry-run')), false);
});
