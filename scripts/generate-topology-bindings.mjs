#!/usr/bin/env node
// Foundation is the only editable source for Camera and Matter Node source DTOs.
import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const args = process.argv.slice(2);
const check = args.includes('--check');
const option = name => { const i = args.indexOf(name); return i < 0 ? null : path.resolve(args[i+1]); };
const read = p => fs.readFileSync(path.join(root,p),'utf8');
const write = (file, value) => {
  if (check) { if (!fs.existsSync(file) || fs.readFileSync(file,'utf8') !== value) throw Error(`Stale generated binding: ${file}`); }
  else {fs.mkdirSync(path.dirname(file),{recursive:true}); fs.writeFileSync(file,value);}
};
const hash = text => createHash('sha256').update(text).digest('hex');
const cameraPath = 'crates/core-api/src/node/contracts/camera_topology.rs';
const camera = read(cameraPath);
const cameraBinding = `// Generated from Foundation ${cameraPath}\n// Source SHA-256: ${hash(camera)}. DO NOT EDIT.\n` + camera;
const baycat = option('--baycat');
if (baycat) write(path.join(baycat,'nodes/camera/src/topology_contract.generated.rs'),cameraBinding);

const matterPath = 'crates/core-api/src/node/contracts/matter_device_sources.rs';
const matter = read(matterPath);
const matterBinding = `// Generated from Foundation ${matterPath}\n// Source SHA-256: ${hash(matter)}. DO NOT EDIT.\n` + matter;
if (baycat) write(path.join(baycat,'nodes/matter/src/device_sources_contract.generated.rs'),matterBinding);

console.log(check?'Topology bindings verified':'Topology bindings generated');
