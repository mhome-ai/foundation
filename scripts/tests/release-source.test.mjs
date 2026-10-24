import assert from 'node:assert/strict';
import {execFileSync, spawnSync} from 'node:child_process';
import {mkdtempSync, mkdirSync, writeFileSync, rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join, resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import test from 'node:test';

const script = fileURLToPath(new URL('../check-release-source.sh', import.meta.url));

test('release source must be reachable from main or a publication branch', () => {
  const root = mkdtempSync(join(tmpdir(), 'foundation-release-source-'));
  const repo = join(root, 'repo');
  const origin = join(root, 'origin.git');
  const env = {...process.env, GIT_CONFIG_GLOBAL: '/dev/null', GIT_CONFIG_NOSYSTEM: '1'};
  const git = (...args) => execFileSync('git', ['-C', repo, ...args], {env, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe']}).trim();
  const check = (sha) => spawnSync('bash', [script, sha], {cwd: repo, env, encoding: 'utf8'});
  try {
    mkdirSync(repo);
    git('init', '-q', '--initial-branch=main');
    git('config', 'user.name', 'Release test');
    git('config', 'user.email', 'release@example.invalid');
    writeFileSync(join(repo, 'source'), 'main\n');
    git('add', '.');
    git('commit', '-qm', 'Main source');
    git('init', '--bare', origin);
    git('remote', 'add', 'origin', resolve(origin));
    git('push', 'origin', 'main');
    assert.equal(check(git('rev-parse', 'HEAD')).status, 0);
    git('checkout', '-qb', 'repo-update/run-1');
    writeFileSync(join(repo, 'source'), 'release\n');
    git('commit', '-qam', 'Prepared release');
    const release = git('rev-parse', 'HEAD');
    assert.notEqual(check(release).status, 0);
    git('push', 'origin', 'HEAD:refs/heads/development/untrusted');
    assert.notEqual(check(release).status, 0);
    git('push', 'origin', 'HEAD:refs/heads/repo-update/run-1');
    assert.equal(check(release).status, 0);
    git('push', 'origin', ':refs/heads/repo-update/run-1');
    assert.notEqual(check(release).status, 0);
    assert.notEqual(check('invalid').status, 0);
  } finally {
    rmSync(root, {recursive: true, force: true});
  }
});
