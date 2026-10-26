#!/usr/bin/env node
// Upload acceptance and Cargo index visibility are separate release states.
import {execFileSync} from 'node:child_process';
import {performance} from 'node:perf_hooks';
import {resolve} from 'node:path';
import {setTimeout as sleep} from 'node:timers/promises';
import {fileURLToPath, pathToFileURL} from 'node:url';

const VERSION = /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/;

export function indexUrl(name) {
  if (!/^[a-zA-Z][a-zA-Z0-9_-]*$/.test(name)) {
    throw new Error(`Invalid crate name: ${name}`);
  }
  const lower = name.toLowerCase();
  const prefix = lower.length < 3 ? `${lower.length}` : lower.length === 3 ? `3/${lower[0]}` : `${lower.slice(0, 2)}/${lower.slice(2, 4)}`;
  return `https://index.crates.io/${prefix}/${lower}`;
}

export async function checkIndex(name, version, {fetchImpl = fetch, timeoutMs = 15000} = {}) {
  const url = indexUrl(name);
  if (!VERSION.test(version)) throw new Error(`Invalid crate version: ${version}`);
  let text;
  let status;
  try {
    const response = await fetchImpl(url, {
      headers: {'Cache-Control': 'no-cache', 'User-Agent': 'mhome-foundation-release/1.0'},
      signal: AbortSignal.timeout(Math.max(1, Math.ceil(timeoutMs))),
    });
    status = response.status;
    // Read/cancel the body within the same request timeout, including HTTP errors.
    if (status === 200) text = await response.text();
    else await response.body?.cancel();
  } catch (error) {
    return {ready: false, reason: `index request failed: ${error.message}`};
  }
  if (status === 404 || status === 408 || status === 429 || status >= 500) {
    return {ready: false, reason: `index HTTP ${status}`};
  }
  if (status !== 200) throw new Error(`${name} ${version}: index HTTP ${status}`);
  let entries;
  try {
    entries = text.split('\n').filter((line) => line.trim()).map((line) => JSON.parse(line));
  } catch {
    throw new Error(`${name} ${version}: invalid Cargo index JSON`);
  }
  if (entries.some((entry) => !entry || typeof entry !== 'object' || entry.name !== name)) {
    throw new Error(`${name} ${version}: index contains an unexpected package`);
  }
  const entry = entries.find((entry) => entry.vers === version);
  if (!entry) return {ready: false, reason: 'exact version is not in the Cargo index yet'};
  if (entry.yanked === true) throw new Error(`${name} ${version} is yanked`);
  if (entry.yanked !== false || !/^[a-f0-9]{64}$/.test(entry.cksum ?? '') || ![1, 2].includes(entry.v ?? 1)) {
    throw new Error(`${name} ${version}: invalid or unsupported Cargo index entry`);
  }
  return {ready: true};
}

export async function waitForCrates(requirements, {
  timeoutMs = 600000, intervalMs = 5000, fetchImpl = fetch,
  now = () => performance.now(), delay = sleep, log = console.log,
} = {}) {
  if (!(timeoutMs > 0) || !(intervalMs > 0)) throw new Error('Wait timeout and interval must be positive');
  const pending = new Map(requirements.map(({name, version}) => [`${name}@${version}`, {name, version}]));
  const reasons = new Map();
  const deadline = now() + timeoutMs;
  while (pending.size) {
    for (const [key, {name, version}] of pending) {
      const remaining = deadline - now();
      if (remaining <= 0) break;
      const result = await checkIndex(name, version, {fetchImpl, timeoutMs: Math.min(15000, remaining)});
      if (result.ready) {
        pending.delete(key);
        log(`Cargo index ready: ${name} ${version}`);
      } else if (reasons.get(key) !== result.reason) {
        reasons.set(key, result.reason);
        log(`Waiting for ${name} ${version}: ${result.reason}`);
      }
    }
    if (!pending.size) return;
    const remaining = deadline - now();
    if (remaining <= 0) {
      throw new Error(`Cargo index readiness timed out after ${timeoutMs / 1000}s: ${[...pending.keys()].map((key) => `${key} (${reasons.get(key) ?? 'not checked before deadline'})`).join(', ')}. Upload records alone do not make a dependency available.`);
    }
    await delay(Math.min(intervalMs, remaining));
  }
}

export function dependencyRequirements(metadata, packageName) {
  const memberIds = new Set(metadata.workspace_members);
  const members = new Map(metadata.packages.filter((pkg) => memberIds.has(pkg.id)).map((pkg) => [pkg.name, pkg]));
  const selected = members.get(packageName);
  if (!selected) throw new Error(`Workspace package not found: ${packageName}`);
  const requirements = [];
  for (const dep of selected.dependencies) {
    if (!dep.path) continue;
    // Foundation publishes its path dependencies to crates.io with exact pins.
    // Keep the declared pin, even if another workspace package has advanced.
    if (!members.has(dep.name) || dep.registry || !dep.req.startsWith('=') || !VERSION.test(dep.req.slice(1))) {
      throw new Error(`${packageName}: unsupported release dependency ${dep.name} ${dep.req}`);
    }
    requirements.push({name: dep.name, version: dep.req.slice(1)});
  }
  return requirements;
}

async function main(args) {
  let requirements;
  if (args.length === 2 && args[0] === '--dependencies-of') {
    const metadata = JSON.parse(execFileSync('cargo', ['metadata', '--format-version', '1', '--no-deps', '--locked', '--offline'], {
      cwd: fileURLToPath(new URL('../', import.meta.url)), encoding: 'utf8', maxBuffer: 16 * 1024 * 1024,
      stdio: ['ignore', 'pipe', 'inherit'], timeout: 60000,
    }));
    requirements = dependencyRequirements(metadata, args[1]);
  } else if (args.length === 2 && !args[0].startsWith('-')) {
    requirements = [{name: args[0], version: args[1]}];
  } else {
    throw new Error('Usage: wait-for-crates.mjs PACKAGE VERSION | --dependencies-of PACKAGE');
  }
  await waitForCrates(requirements);
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  main(process.argv.slice(2)).catch((error) => {
    console.error(error.message);
    process.exitCode = 1;
  });
}
