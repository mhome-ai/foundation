import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const source = path.join(root, "crates/core-api/manifest/operation-timeouts.v1.json");
const output = path.join(root, "crates/core-api/src/timeouts.generated.rs");
const snake = value => value.replace(/[A-Z]/g, c => `_${c.toLowerCase()}`).toUpperCase();
const variant = value => value[0].toUpperCase() + value.slice(1);
const literal = value => JSON.stringify(value);

export function validateTimeouts(data) {
  const keys = (object, allowed, required = allowed) => {
    if (!object || typeof object !== "object" || Array.isArray(object)
        || Object.keys(object).some(key => !allowed.includes(key))
        || required.some(key => !(key in object))) throw new Error("Invalid timeout catalog fields");
  };
  const millis = (value, zero = false) => {
    if (!Number.isSafeInteger(value) || value < (zero ? 0 : 1) || value > 2147483647)
      throw new Error("Invalid timeout milliseconds");
  };
  keys(data, ["manifest", "operations", "waitProfiles", "selectors"]);
  if (data.manifest !== "mhome.operation-timeouts.v1") throw new Error("Unknown timeout catalog");
  for (const [id, policy] of Object.entries(data.operations)) {
    if (!/^[a-z][a-zA-Z0-9]*$/.test(id)) throw new Error("Invalid operation ID");
    keys(policy, ["totalMs", "description"]);
    millis(policy.totalMs);
    if (typeof policy.description !== "string" || !policy.description.trim()) throw new Error("Missing description");
  }
  if (!data.operations.coreRequest || !data.operations.scriptSave || !data.operations.scriptValidation)
    throw new Error("Missing ingress policy");
  for (const [id, profile] of Object.entries(data.waitProfiles)) {
    if (!/^[a-z][a-zA-Z0-9]*$/.test(id)) throw new Error("Invalid wait profile ID");
    keys(profile, ["graceMs", "overrides"], ["graceMs"]);
    millis(profile.graceMs, true);
    for (const [operation, grace] of Object.entries(profile.overrides ?? {})) {
      if (!data.operations[operation]) throw new Error("Unknown wait operation");
      millis(grace, true);
    }
    for (const [operation, policy] of Object.entries(data.operations))
      millis(policy.totalMs + (profile.overrides?.[operation] ?? profile.graceMs));
  }
  keys(data.selectors, ["forwardPrefixes", "storedScriptSave", "sourceScriptSave", "sourceScriptValidation", "ordinaryOverrides"]);
  const targets = new Set();
  for (const [kind, values] of Object.entries(data.selectors)) {
    if (kind === "ordinaryOverrides") continue;
    if (!Array.isArray(values) || new Set(values).size !== values.length
        || values.some(value => typeof value !== "string" || !value.startsWith("/")))
      throw new Error("Invalid selector targets");
    if (kind !== "forwardPrefixes") for (const value of values) {
      if (targets.has(value)) throw new Error("Overlapping timeout selectors");
      targets.add(value);
    }
  }
  for (const [target, operation] of Object.entries(data.selectors.ordinaryOverrides))
    if (!target.startsWith("/") || !data.operations[operation]) throw new Error("Invalid timeout override");
}

export function renderTimeouts(data) {
  validateTimeouts(data);
  const operations = Object.entries(data.operations);
  const profiles = Object.entries(data.waitProfiles);
  return `// Generated from manifest/operation-timeouts.v1.json. DO NOT EDIT.\n\n`
    + `#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub enum Operation {\n${operations.map(([id]) => `    ${variant(id)},`).join("\n")}\n}\n\n`
    + `impl Operation {\n    pub const fn id(self) -> &'static str {\n        match self {\n${operations.map(([id]) => `            Self::${variant(id)} => ${literal(id)},`).join("\n")}\n        }\n    }\n\n`
    + `    pub const fn timeout(self) -> Duration {\n        Duration::from_millis(match self {\n${operations.map(([id, p]) => `            Self::${variant(id)} => ${p.totalMs},`).join("\n")}\n        })\n    }\n}\n\n`
    + `#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub enum WaitProfile {\n${profiles.map(([id]) => `    ${variant(id)},`).join("\n")}\n}\n\n`
    + `impl WaitProfile {\n    pub const fn grace(self, operation: Operation) -> Duration {\n        Duration::from_millis(match (self, operation) {\n${profiles.flatMap(([id, p]) => [...Object.entries(p.overrides ?? {}).map(([op, value]) => `            (Self::${variant(id)}, Operation::${variant(op)}) => ${value},`), `            (Self::${variant(id)}, _) => ${p.graceMs},`]).join("\n")}\n        })\n    }\n}\n\n`
    + Object.entries(data.selectors).filter(([id]) => id !== "ordinaryOverrides").map(([id, values]) => `const ${snake(id)}: &[&str] = &[\n${values.map(value => `    ${literal(value)},`).join("\n")}\n];\n`).join("\n")
    + `\nconst ORDINARY_OVERRIDES: &[(&str, Operation)] = &[\n${Object.entries(data.selectors.ordinaryOverrides).map(([target, op]) => `    (${literal(target)}, Operation::${variant(op)}),`).join("\n")}\n];\n`;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const rendered = renderTimeouts(JSON.parse(fs.readFileSync(source, "utf8")));
  if (process.argv[2] === "--write") fs.writeFileSync(output, rendered);
  else if (process.argv[2] === "--check") {
    if (fs.readFileSync(output, "utf8") !== rendered) throw new Error("Timeout bindings need regeneration");
  } else throw new Error("Use --write or --check");
}
