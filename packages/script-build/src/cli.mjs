#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { buildIntegration } from "./integration.mjs";

// Developer/CI entrypoint. It checks and bundles sources without evaluating them.
try {
  const args = process.argv.slice(2);
  if (args.length !== 3) throw new Error("Usage: mhome-script-build <project-directory> <functions-json-array> <empty-output-directory>");
  const [projectRoot, names, directory] = args;
  const output = path.resolve(directory);
  if (fs.existsSync(output) && fs.readdirSync(output).length) throw new Error("Output directory must be empty");
  const builds = await buildIntegration({ projectRoot, functions: JSON.parse(names) });
  fs.mkdirSync(output, { recursive: true });
  const functions = [];
  for (const { javascript, ...metadata } of builds) {
    fs.writeFileSync(path.join(output, `${metadata.function}.js`), javascript, { flag: "wx" });
    functions.push(metadata);
  }
  fs.writeFileSync(path.join(output, "build.json"), JSON.stringify({ functions }, null, 2) + "\n", { flag: "wx" });
} catch (error) {
  process.stderr.write(JSON.stringify({ code: error.code ?? "BUILD_FAILED",
    message: String(error.message).slice(0, 8192), diagnostics: error.diagnostics ?? [] }) + "\n");
  process.exitCode = 1;
}
