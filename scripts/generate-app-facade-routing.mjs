import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const crateRoot = path.join(root, "crates", "app-facade-api");
const manifestPath = path.join(crateRoot, "manifest", "routing.v1.json");
const outputPath = path.join(crateRoot, "src", "routing.generated.rs");

const RULE_FIELDS = Object.freeze([
  "cloudPrefixes",
  "cloudTargets",
  "scopeModeTargets",
  "hubPrefixes",
  "hostTargets",
  "requestPlacementPrefixes",
  "cloudRelayTargets",
]);
const EXACT_FIELDS = Object.freeze([
  "cloudTargets",
  "scopeModeTargets",
  "hostTargets",
  "cloudRelayTargets",
]);
const PREFIX_FIELDS = Object.freeze([
  "cloudPrefixes",
  "hubPrefixes",
  "requestPlacementPrefixes",
]);

function assertRoutingContract(manifest) {
  if (
    manifest.manifest !== "mhome.app-facade.routing.v1" ||
    manifest.targetPrefix !== "/app/" ||
    manifest.default?.execution !== "scopeMode" ||
    manifest.default?.relay !== "directOnly" ||
    manifest.matching !== "exactBeforePrefix" ||
    manifest.placementJsonPointer !== "/input/placement"
  ) {
    throw new Error("Unsupported App Facade routing contract metadata");
  }

  const rules = manifest.rules ?? {};
  if (
    Object.keys(rules).length !== RULE_FIELDS.length ||
    Object.keys(rules).some((field) => !RULE_FIELDS.includes(field))
  ) {
    throw new Error("App Facade routing contract has unknown or missing rule groups");
  }
  for (const field of RULE_FIELDS) {
    if (
      !Array.isArray(rules[field]) ||
      new Set(rules[field]).size !== rules[field].length ||
      rules[field].some(
        (target) => typeof target !== "string" || !target.startsWith(manifest.targetPrefix),
      )
    ) {
      throw new Error(`App Facade routing rule ${field} is malformed`);
    }
  }

  const exactOwners = new Map();
  for (const field of EXACT_FIELDS) {
    for (const target of rules[field]) {
      if (exactOwners.has(target)) {
        throw new Error(
          `Exact route ${target} is declared by both ${exactOwners.get(target)} and ${field}`,
        );
      }
      exactOwners.set(target, field);
    }
  }

  const prefixes = PREFIX_FIELDS.flatMap((field) =>
    rules[field].map((prefix) => ({ field, prefix })),
  );
  for (let index = 0; index < prefixes.length; index += 1) {
    for (let other = index + 1; other < prefixes.length; other += 1) {
      const left = prefixes[index];
      const right = prefixes[other];
      if (left.prefix.startsWith(right.prefix) || right.prefix.startsWith(left.prefix)) {
        throw new Error(
          `Routing prefixes ${left.prefix} (${left.field}) and ${right.prefix} (${right.field}) overlap`,
        );
      }
    }
  }
}

const rustString = (value) => JSON.stringify(value);
const rustSlice = (name, values, visibility = "const") => {
  const entries = values.map((value) => `    ${rustString(value)},`).join("\n");
  return `${visibility} ${name}: &[&str] = &[\n${entries}\n];`;
};

export function renderAppFacadeRouting(manifest) {
  assertRoutingContract(manifest);
  const rules = manifest.rules;
  return `// Generated from manifest/routing.v1.json. DO NOT EDIT.

const TARGET_PREFIX: &str = ${rustString(manifest.targetPrefix)};

${rustSlice("CLOUD_PREFIXES", rules.cloudPrefixes)}

/// Exact Cloud-owned operations outside Cloud-owned domains.
${rustSlice("CLOUD_TARGETS", rules.cloudTargets, "pub const")}

const SCOPE_MODE_TARGETS: &[&str] = &[
${rules.scopeModeTargets.map((value) => `    ${rustString(value)},`).join("\n")}
];

const HUB_PREFIXES: &[&str] = &[
${rules.hubPrefixes.map((value) => `    ${rustString(value)},`).join("\n")}
];

const HOST_TARGETS: &[&str] = &[
${rules.hostTargets.map((value) => `    ${rustString(value)},`).join("\n")}
];

const REQUEST_PLACEMENT_PREFIXES: &[&str] = &[
${rules.requestPlacementPrefixes.map((value) => `    ${rustString(value)},`).join("\n")}
];

/// Hub management operations for which Lion provides a transport relay.
${rustSlice("CLOUD_RELAY_TARGETS", rules.cloudRelayTargets, "pub const")}
`;
}

export function generateAppFacadeRouting({ write = false } = {}) {
  const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
  const generated = renderAppFacadeRouting(manifest);
  if (write) {
    fs.writeFileSync(outputPath, generated);
    return;
  }
  const current = fs.readFileSync(outputPath, "utf8");
  if (current !== generated) {
    throw new Error(
      "crates/app-facade-api/src/routing.generated.rs is stale; run npm run generate:app-facade-routing",
    );
  }
}

const mode = process.argv[2] ?? "--check";
if (path.resolve(process.argv[1] || "") === fileURLToPath(import.meta.url)) {
  if (mode !== "--check" && mode !== "--write") {
    throw new Error("usage: node scripts/generate-app-facade-routing.mjs [--check|--write]");
  }
  generateAppFacadeRouting({ write: mode === "--write" });
}
