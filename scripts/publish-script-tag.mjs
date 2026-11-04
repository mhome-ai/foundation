import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { execFileSync } from "node:child_process";
import { SCRIPT_PACKAGES, scriptReleasePlan } from "./script-packages.mjs";
import { stageScriptPackage } from "./stage-script-package.mjs";
import { uploadNpm } from "./upload-npm.mjs";

const root = fileURLToPath(new URL("../", import.meta.url));

export function publishScriptTag(tag, {
  publish = false, verifyPublished = false, env = process.env, source = root, stage = stageScriptPackage,
  run = (command, args) => execFileSync(command, args, { cwd: source, env, encoding: "utf8", stdio: ["ignore", "pipe", "inherit"] }),
} = {}) {
  if (publish && verifyPublished) throw new Error("Publication and read-only verification are mutually exclusive");
  const selected = scriptReleasePlan(tag, source);
  if (publish && (env.GITHUB_ACTIONS !== "true" || env.RUNNER_ENVIRONMENT !== "github-hosted"
      || env.GITHUB_REF !== `refs/tags/${tag}` || env.GITHUB_REPOSITORY !== "mhome-ai/foundation"
      || env.GITHUB_WORKFLOW_REF !== `mhome-ai/foundation/.github/workflows/publish-crate.yml@refs/tags/${tag}`)) {
    throw new Error("Script publication requires the tagged GitHub-hosted Foundation workflow");
  }
  if (publish && run("npm", ["--version"]).trim() !== "12.2.1") throw new Error("Publication requires npm 12.2.1");
  // Build both packages and exercise their independent packed installation before any upload.
  for (const item of Object.values(SCRIPT_PACKAGES)) {
    run("npm", ["ci", "--prefix", `packages/${item.directory}`, "--ignore-scripts", "--no-audit", "--no-fund"]);
    run("npm", ["test", "--prefix", `packages/${item.directory}`]);
  }
  const temporary = fs.mkdtempSync(path.join(os.tmpdir(), "foundation-script-release-"));
  try {
    const artifacts = {};
    for (const kind of Object.keys(SCRIPT_PACKAGES)) {
      const directory = path.join(temporary, kind);
      const manifest = stage(kind, directory);
      const packed = JSON.parse(run("npm", ["pack", directory, "--json", "--ignore-scripts", "--pack-destination", temporary]));
      const result = Array.isArray(packed) ? packed[0] : Object.values(packed)[0];
      if (result.name !== manifest.name || result.version !== manifest.version || path.basename(result.filename) !== result.filename) {
        throw new Error("Packed script package identity disagrees with its manifest");
      }
      const tarball = path.join(temporary, result.filename);
      if (!fs.statSync(tarball).isFile()) throw new Error("Missing script package archive");
      artifacts[kind] = { manifest, tarball };
    }
    if (!publish && !verifyPublished) return selected;
    const registry = (action, item) => run("python3", ["-B", "scripts/release-registry.py", action,
      "npm", item.manifest.name, item.manifest.version, ...(action === "verify" ? [item.tarball] : [])]);
    // The builder must consume the already published, identical SDK, never just a matching version label.
    if (selected.kind === "build") registry("verify", artifacts.sdk);
    const item = artifacts[selected.kind];
    if (verifyPublished) {
      registry("verify", item);
      return selected;
    }
    let exists = false;
    try { registry("exists", item); exists = true; }
    catch (error) { if (error.status !== 3) throw error; }
    if (exists) {
      registry("verify", item);
      return selected;
    }
    const proof = path.join(temporary, "provenance.sigstore.json");
    run("node", ["scripts/protocol-provenance.mjs", selected.name, selected.version, item.tarball, proof]);
    uploadNpm(item.tarball, proof, { run });
    registry("wait", item);
    registry("verify", item);
    return selected;
  } finally {
    fs.rmSync(temporary, { recursive: true, force: true });
  }
}

if (path.resolve(process.argv[1] || "") === fileURLToPath(import.meta.url)) {
  const [tag, mode] = process.argv.slice(2);
  if (!tag || (mode !== undefined && !["--publish", "--verify-published"].includes(mode)) || process.argv.length > 4) throw new Error("Usage: publish-script-tag.mjs <tag> [--publish | --verify-published]");
  const result = publishScriptTag(tag, { publish: mode === "--publish", verifyPublished: mode === "--verify-published" });
  console.log(`${mode === "--publish" ? "Published and verified" : mode === "--verify-published" ? "Verified against registry" : "Verified locally"} ${result.name}@${result.version}`);
}
