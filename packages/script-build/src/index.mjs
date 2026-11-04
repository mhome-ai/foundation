import { createHash } from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import ts from "typescript";
import * as esbuild from "esbuild";
import { SDK_VERSION } from "@mhome/script";
import { BuildError, readContract } from "./contract.mjs";
export { BuildError } from "./contract.mjs";
export { buildIntegration } from "./integration.mjs";

const root = fileURLToPath(new URL("..", import.meta.url));
const sdkEntry = fileURLToPath(import.meta.resolve("@mhome/script"));
const sdkDirectory = path.dirname(sdkEntry);
const typeScriptDirectory = path.dirname(fileURLToPath(import.meta.resolve("typescript")));
const HOST_MODULE = "@mhome/script/host";
const PROFILE = "automation-es2022-v1";
const MAX_SOURCE_BYTES = 256 * 1024;
const MAX_ARTIFACT_BYTES = 4 * 1024 * 1024;
const hash = (value) => createHash("sha256").update(value).digest("hex");
const failure = (code, message) => { throw new BuildError(code, [{ message }]); };

function checkTypes(filename, source) {
  const options = {
    target: ts.ScriptTarget.ES2022,
    module: ts.ModuleKind.ESNext,
    moduleResolution: ts.ModuleResolutionKind.Bundler,
    lib: ["lib.es2022.d.ts"],
    types: [],
    strict: true,
    exactOptionalPropertyTypes: true,
    noUncheckedIndexedAccess: true,
    noEmit: true,
    skipLibCheck: false,
  };
  const host = ts.createCompilerHost(options);
  const readFile = host.readFile.bind(host);
  const fileExists = host.fileExists.bind(host);
  const allowed = (name) => name === filename
    || (path.dirname(name) === typeScriptDirectory && /^lib\..*\.d\.ts$/.test(path.basename(name)))
    || (name.startsWith(`${sdkDirectory}${path.sep}`) && name.endsWith(".d.ts") && !path.relative(sdkDirectory, name).startsWith(".."));
  host.readFile = (name) => name === filename ? source : allowed(name) ? readFile(name) : undefined;
  host.fileExists = (name) => name === filename || (allowed(name) && fileExists(name));
  // Use this host's reader, including the in-memory author source.
  host.getSourceFile = (name, languageVersion) => {
    const text = host.readFile(name);
    return text === undefined ? undefined : ts.createSourceFile(name, text, languageVersion, true);
  };
  host.resolveModuleNames = (names, containingFile) => names.map((name) => {
    let resolvedFileName;
    if (name === "@mhome/script") resolvedFileName = path.join(sdkDirectory, "index.d.ts");
    else if (name === HOST_MODULE) resolvedFileName = path.join(sdkDirectory, "host.d.ts");
    else if (containingFile.startsWith(`${sdkDirectory}${path.sep}`) && name.startsWith(".")) {
      resolvedFileName = path.resolve(path.dirname(containingFile), name.replace(/\.js$/, ".d.ts"));
    }
    if (!resolvedFileName || !allowed(resolvedFileName)) return undefined;
    return { resolvedFileName, extension: ts.Extension.Dts, isExternalLibraryImport: true };
  });
  const program = ts.createProgram([filename], options, host);
  const diagnostics = ts.getPreEmitDiagnostics(program);
  if (diagnostics.length) {
    let remaining = 16 * 1024;
    const bounded = diagnostics.slice(0, 50).map((diagnostic) => {
      const message = ts.flattenDiagnosticMessageText(diagnostic.messageText, "\n").slice(0, remaining);
      remaining = Math.max(0, remaining - message.length);
      const result = { message, code: diagnostic.code };
      if (diagnostic.file) {
        const { line, character } = diagnostic.file.getLineAndCharacterOfPosition(diagnostic.start ?? 0);
        Object.assign(result, { file: path.basename(diagnostic.file.fileName), line: line + 1, column: character + 1 });
      }
      return result;
    });
    throw new BuildError("TYPE_CHECK_FAILED", bounded);
  }
  return program.getSourceFile(filename);
}

/**
 * Compile one Automation source in memory. No source execution, npm install,
 * user tsconfig/build plugins, filesystem imports, or runtime package fetching.
 * A service must run this CPU work in a bounded build worker, with an outer
 * deadline; AbortSignal alone cannot preempt TypeScript's synchronous checker.
 */
export async function buildAutomation({ function: name, source, sdkVersion = SDK_VERSION, signal } = {}) {
  if (typeof name !== "string" || !/^[A-Za-z_][A-Za-z0-9_]{0,127}$/.test(name)) failure("INVALID_SOURCE", "Function must be an ASCII identifier without a path or extension");
  if (typeof source !== "string" || !source.length || Buffer.byteLength(source) > MAX_SOURCE_BYTES) failure("INVALID_SOURCE", "TypeScript source must contain 1 to 256 KiB of UTF-8 text");
  if (sdkVersion !== SDK_VERSION) failure("UNSUPPORTED_SDK", `This builder requires SDK ${SDK_VERSION}`);
  signal?.throwIfAborted();
  const filename = path.join(root, "virtual", `${name}.ts`);
  const sourceFile = checkTypes(filename, source);
  const contract = readContract(sourceFile, sdkVersion);
  signal?.throwIfAborted();
  let result;
  try {
    result = await esbuild.build({
      stdin: { contents: source, loader: "ts", sourcefile: `${name}.ts`, resolveDir: root },
      absWorkingDir: root,
      bundle: true,
      format: "esm",
      platform: "neutral",
      target: "es2022",
      write: false,
      logLevel: "silent",
      legalComments: "none",
      minifyWhitespace: true,
      keepNames: true,
      plugins: [{ name: "managed-script-imports", setup(build) {
        build.onResolve({ filter: /.*/ }, (args) => {
          // Native module evaluation only declares bindings. Drop unused imports
          // so pure scripts do not require host bindings for side effects.
          if (args.path === HOST_MODULE) return { path: args.path, external: true, sideEffects: false };
          if (args.path === "@mhome/script") return { path: sdkEntry };
          return { errors: [{ text: `Unsupported Automation import: ${args.path}` }] };
        });
      } }],
    });
  } catch (error) {
    const diagnostics = (error.errors ?? []).slice(0, 50).map((entry) => ({ message: entry.text.slice(0, 4096),
      ...(entry.location ? { file: path.basename(entry.location.file), line: entry.location.line, column: entry.location.column + 1 } : {}) }));
    throw new BuildError("BUNDLE_FAILED", diagnostics.length ? diagnostics : [{ message: "JavaScript bundling failed" }]);
  }
  signal?.throwIfAborted();
  const javascript = result.outputFiles[0].text;
  const size = Buffer.byteLength(javascript);
  if (!size || size > MAX_ARTIFACT_BYTES) failure("ARTIFACT_TOO_LARGE", "Bundled JavaScript exceeds 4 MiB");
  const sdkHash = hash([sdkEntry, path.join(sdkDirectory, "index.d.ts"), path.join(sdkDirectory, "host.d.ts")]
    .map((name) => fs.readFileSync(name, "utf8")).join("\0"));
  const build = { profile: PROFILE, sdkVersion, sdkHash, typescript: ts.version, esbuild: esbuild.version };
  const sourceHash = hash(JSON.stringify({ function: name, source, build }));
  return {
    function: name, sourceHash, build, javascript,
    artifact: { format: "javascript-esm", sha256: hash(javascript), size, sdkVersion, contract },
  };
}
