import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

// Explicit destinations permit isolated worktrees. Without --write, verify only.
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const source = fs.readFileSync(path.join(root, "crates/app-facade-api/manifest/plugin-catalog.v1.json"), "utf8");
const outputs = process.argv.slice(2).filter(arg => arg !== "--write");
if (!outputs.length) throw new Error("Supply Core and Cloud catalog resource file paths");
for (const output of outputs) {
  if (process.argv.includes("--write")) fs.writeFileSync(output, source);
  if (fs.readFileSync(output, "utf8") !== source) throw new Error(`Stale plugin catalog: ${output}`);
}
