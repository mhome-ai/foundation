import fs from "node:fs/promises";
import { buildAutomation } from "../src/index.mjs";

const directory = new URL("../../../crates/script-runtime/fixtures/", import.meta.url);
const source = await fs.readFile(new URL("lighting.ts", directory), "utf8");
const { javascript, ...metadata } = await buildAutomation({ function: "lighting", source });
await fs.writeFile(new URL("lighting.js", directory), javascript);
await fs.writeFile(new URL("lighting.build.json", directory), `${JSON.stringify(metadata, null, 2)}\n`);
