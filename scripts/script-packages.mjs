import fs from "node:fs";
import path from "node:path";

export const SCRIPT_PACKAGES = Object.freeze({
  sdk: { name: "@mhome/script", directory: "script-sdk", tag: "mhome-script-v" },
  build: { name: "@mhome/script-build", directory: "script-build", tag: "mhome-script-build-v" },
});

export function scriptReleasePlan(tag, root) {
  const entry = Object.entries(SCRIPT_PACKAGES).find(([, item]) => tag.startsWith(item.tag));
  if (!entry) throw new Error(`Unsupported script release tag: ${tag}`);
  const [kind, info] = entry;
  const version = tag.slice(info.tag.length);
  if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(version)) throw new Error("Invalid script release version");
  const manifest = JSON.parse(fs.readFileSync(path.join(root, "packages", info.directory, "package.json")));
  if (manifest.name !== info.name || manifest.version !== version) throw new Error("Script release tag and package identity/version disagree");
  return { ...info, kind, version };
}
