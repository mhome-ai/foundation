import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));
const projects = { sdk: "script-sdk", build: "script-build" };

/** Source workspaces use file links; distributable packages use exact registry versions. */
export function stageScriptPackage(kind, directory) {
  const project = projects[kind];
  if (!project) throw new Error("Script package must be sdk or build");
  const source = path.join(root, "packages", project);
  const manifest = JSON.parse(fs.readFileSync(path.join(source, "package.json"), "utf8"));
  const output = path.resolve(directory);
  if (fs.existsSync(output) && fs.readdirSync(output).length) throw new Error("Staging directory must be empty");
  const files = kind === "sdk" ? ["dist/index.js", "dist/index.d.ts", "dist/host.js", "dist/host.d.ts"]
    : ["src/index.mjs", "src/integration.mjs", "src/contract.mjs", "src/cli.mjs"];
  // The SDK must be built first by the normal TypeScript build, never by an install hook.
  for (const file of files) {
    const input = path.join(source, file);
    if (!fs.statSync(input).isFile() || fs.lstatSync(input).isSymbolicLink()) throw new Error(`Invalid package input: ${file}`);
  }
  fs.mkdirSync(output, { recursive: true });
  for (const file of [...files, "README.md"]) {
    const destination = path.join(output, file);
    fs.mkdirSync(path.dirname(destination), { recursive: true });
    fs.copyFileSync(path.join(source, file), destination);
  }
  fs.copyFileSync(path.join(root, "LICENSE"), path.join(output, "LICENSE"));
  const { name, version, type, description, exports } = manifest;
  const packaged = { name, version, type, description, exports, license: "MIT",
    engines: { node: ">=22" }, files, publishConfig: { access: "public" },
    repository: { type: "git", url: "git+https://github.com/mhome-ai/foundation.git", directory: `packages/${project}` } };
  if (kind === "build") {
    const sdk = JSON.parse(fs.readFileSync(path.join(root, "packages/script-sdk/package.json"), "utf8"));
    packaged.dependencies = { ...manifest.dependencies, [sdk.name]: sdk.version };
    packaged.bin = { "mhome-script-build": "src/cli.mjs" };
    fs.chmodSync(path.join(output, "src/cli.mjs"), 0o755);
  }
  fs.writeFileSync(path.join(output, "package.json"), JSON.stringify(packaged, null, 2) + "\n");
  return packaged;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  if (process.argv.length !== 4) throw new Error("Usage: node scripts/stage-script-package.mjs <sdk|build> <empty-output-directory>");
  stageScriptPackage(process.argv[2], process.argv[3]);
}
