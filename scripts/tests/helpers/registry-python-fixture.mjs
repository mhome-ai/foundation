import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';

// Resolve before any fixture prepends its command directory to PATH.
const python = execFileSync('python3', ['-c', 'import sys; print(sys.executable)'], {
  encoding: 'utf8', timeout: 5000,
}).trim();
const registry = fileURLToPath(new URL('../../release-registry.py', import.meta.url));

// registryBody can inspect args and operation, and select the registry result.
// Inline Python remains real, including stdin, JSON decoding and exit status.
export function registryPythonFixture(registryBody) {
  return `#!${process.execPath}
const {spawnSync} = require('node:child_process');
const args = process.argv.slice(2);
if (args[0] === '-c' && args.length === 2) {
  const result = spawnSync(${JSON.stringify(python)}, args, {stdio: 'inherit', timeout: 10000});
  if (result.error) console.error(result.error.message);
  process.exit(result.status ?? 1);
}
const operation = args[2];
if (args[0] !== '-B' || args[1] !== ${JSON.stringify(registry)} ||
    !['exists', 'verify', 'wait'].includes(operation) ||
    !['crate', 'npm'].includes(args[3]) || !args[4] || !args[5] ||
    args.length !== (operation === 'verify' ? 7 : 6) ||
    (operation === 'verify' && !args[6])) {
  console.error('Unexpected Python invocation in publication fixture: ' + JSON.stringify(args));
  process.exit(2);
}
${registryBody}
process.exit(0);
`;
}
