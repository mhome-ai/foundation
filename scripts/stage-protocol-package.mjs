import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { protocolPackage } from "./protocol-packages.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

function readJson(file) {
  return JSON.parse(fs.readFileSync(file, "utf8"));
}

function crateVersion(crateDir) {
  const cargoToml = fs.readFileSync(path.join(root, crateDir, "Cargo.toml"), "utf8");
  const version = cargoToml.match(/^version\s*=\s*"([^"]+)"\s*$/m)?.[1];
  if (!version) throw new Error(`Missing package version in ${crateDir}/Cargo.toml`);
  return version;
}

function copyRelative(crateRoot, stagingRoot, relativePath) {
  const source = path.join(crateRoot, relativePath);
  if (!fs.existsSync(source)) {
    throw new Error(`Missing protocol artifact: ${source}`);
  }
  const destination = path.join(stagingRoot, relativePath);
  fs.mkdirSync(path.dirname(destination), { recursive: true });
  fs.cpSync(source, destination, { recursive: true });
}

export function stageProtocolPackage(name, outputDir) {
  const config = protocolPackage(name);
  const crateRoot = path.join(root, config.crateDir);
  const version = crateVersion(config.crateDir);
  const manifest = readJson(path.join(crateRoot, config.manifest));
  if (
    Object.prototype.hasOwnProperty.call(manifest, "contractVersion") &&
    manifest.contractVersion !== version
  ) {
    throw new Error(
      `${config.manifest} contractVersion ${manifest.contractVersion} does not match crate ${version}`
    );
  }

  const stagingRoot = path.resolve(outputDir);
  if (fs.existsSync(stagingRoot) && fs.readdirSync(stagingRoot).length > 0) {
    throw new Error(
      `Refusing to overwrite non-empty staging directory: ${stagingRoot}`
    );
  }
  fs.mkdirSync(stagingRoot, { recursive: true });

  const paths = new Set([
    config.manifest,
    ...Object.values(config.schemas),
    ...Object.values(config.fixtures),
    "README.md",
    "LICENSE",
  ]);
  for (const relativePath of paths) {
    copyRelative(crateRoot, stagingRoot, relativePath);
  }

  const descriptor = {
    protocol: config.protocol,
    contractVersion: version,
    manifest: config.manifest,
    schemas: config.schemas,
    fixtures: config.fixtures,
  };
  const packageJson = {
    name: config.npmName,
    version,
    description: `Build-time protocol artifacts for ${config.protocol}`,
    license: "MIT",
    repository: {
      type: "git",
      url: "git+https://github.com/mhome-ai/foundation.git",
      directory: config.crateDir,
    },
    homepage: "https://github.com/mhome-ai/foundation#readme",
    bugs: {
      url: "https://github.com/mhome-ai/foundation/issues",
    },
    keywords: ["mhome", "protocol", "json-schema", "fixtures"],
    private: false,
    files: [
      "protocol.json",
      "manifest",
      "contract",
      "schema",
      "fixtures",
      "README.md",
      "LICENSE",
    ],
    exports: {
      "./package.json": "./package.json",
      "./protocol.json": "./protocol.json",
      "./manifest/*": "./manifest/*",
      "./contract/*": "./contract/*",
      "./schema/*": "./schema/*",
      "./fixtures/*": "./fixtures/*",
    },
    publishConfig: { access: "public", provenance: true },
  };
  fs.writeFileSync(
    path.join(stagingRoot, "protocol.json"),
    `${JSON.stringify(descriptor, null, 2)}\n`
  );
  fs.writeFileSync(
    path.join(stagingRoot, "package.json"),
    `${JSON.stringify(packageJson, null, 2)}\n`
  );
  return { config, descriptor, packageJson, stagingRoot };
}

function main() {
  const [name, outputDir] = process.argv.slice(2);
  if (!name || !outputDir) {
    throw new Error(
      "usage: node scripts/stage-protocol-package.mjs <conversation|messaging|plugin> <output-dir>"
    );
  }
  const staged = stageProtocolPackage(name, outputDir);
  process.stdout.write(
    `${staged.packageJson.name}@${staged.packageJson.version} staged at ${staged.stagingRoot}\n`
  );
}

if (path.resolve(process.argv[1] || "") === fileURLToPath(import.meta.url)) {
  main();
}
