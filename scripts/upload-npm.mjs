import path from "node:path";
import { fileURLToPath } from "node:url";
import { execFileSync } from "node:child_process";

// These responses reject the write. Network/server errors can instead lose the
// response after a successful upload, so callers must still read the registry.
const rejected = new Set(["E401", "E402", "E403", "E404", "ENEEDAUTH", "EOTP"]);

export function uploadNpm(tarball, proof, {
  run = (command, args) => execFileSync(command, args, { encoding: "utf8", stdio: ["ignore", "pipe", "inherit"] }),
  log = console.error,
} = {}) {
  try {
    run("npm", ["publish", tarball, "--json", "--access", "public", "--provenance-file", proof, "--tag", "latest"]);
  } catch (error) {
    if (error.status === undefined) throw error;
    let code;
    try { code = JSON.parse(String(error.stdout)).error?.code; } catch { /* Unknown result still needs readback. */ }
    if (rejected.has(code)) {
      throw new Error(`npm rejected publication (${code}); check package ownership, first-package registration and trusted publisher permissions.`, { cause: error });
    }
    log("npm upload result is uncertain; checking registry contents without another upload");
  }
}

if (path.resolve(process.argv[1] || "") === fileURLToPath(import.meta.url)) {
  if (process.argv.length !== 4) throw new Error("Usage: upload-npm.mjs <tarball> <provenance-file>");
  uploadNpm(process.argv[2], process.argv[3]);
}
