import { createHash } from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import ts from "typescript";
import * as esbuild from "esbuild";
import { SDK_VERSION } from "@meowlink/script";
import { BuildError } from "./contract.mjs";

const HOST_MODULE = "@meowlink/script/host";
const sdkEntry = fileURLToPath(import.meta.resolve("@meowlink/script"));
const sdkDirectory = path.dirname(sdkEntry);
const tsDirectory = path.dirname(fileURLToPath(import.meta.resolve("typescript")));
const hash = (value) => createHash("sha256").update(value).digest("hex");
const fail = (code, message) => { throw new BuildError(code, [{ message }]); };
const inside = (root, name) => name === root || name.startsWith(`${root}${path.sep}`);
const sorted = (object = {}) => Object.entries(object).sort(([a], [b]) => a.localeCompare(b));
const isFile = (filename) => fs.existsSync(filename) && fs.statSync(filename).isFile();

function checkImports(filename, contents) {
  const source = ts.createSourceFile(filename, contents, ts.ScriptTarget.ES2022, true);
  const visit = (node) => {
    if (ts.isCallExpression(node)
        && (node.expression.kind === ts.SyntaxKind.ImportKeyword
          || (ts.isIdentifier(node.expression) && node.expression.text === "require"))
        && (node.arguments.length !== 1 || !ts.isStringLiteralLike(node.arguments[0]))) {
      fail("BUNDLE_FAILED", "Runtime imports must use literal module names so every dependency can be bundled");
    }
    ts.forEachChild(node, visit);
  };
  visit(source);
}

/**
 * Build an installed Integration function project. The Dev Kit prepares dependencies
 * with npm ci --ignore-scripts; this compiler never installs or executes author code.
 * Only the selected entries, common modules and locked packages enter the bundle.
 */
export async function buildIntegration({ projectRoot, functions, sdkVersion = SDK_VERSION, signal } = {}) {
  if (sdkVersion !== SDK_VERSION) fail("UNSUPPORTED_SDK", `This builder requires SDK ${SDK_VERSION}`);
  if (!Array.isArray(functions) || functions.length > 100 || new Set(functions).size !== functions.length
      || functions.some((name) => typeof name !== "string" || !/^[A-Za-z_][A-Za-z0-9_]{0,127}$/.test(name))) {
    fail("INVALID_SOURCE", "Provide at most 100 unique function identifiers without paths or extensions");
  }
  if (typeof projectRoot !== "string") fail("INVALID_SOURCE", "An Integration function project root is required");
  const root = fs.realpathSync(projectRoot);
  signal?.throwIfAborted();
  const inputs = new Map();
  let totalBytes = 0;
  const read = (filename) => {
    if (inputs.has(filename)) return inputs.get(filename);
    if (!inside(root, filename) && !inside(sdkDirectory, filename) && !inside(tsDirectory, filename)) return undefined;
    if (!fs.existsSync(filename)) return undefined;
    const stat = fs.statSync(filename);
    if (!stat.isFile()) return undefined;
    if (stat.size > 4 * 1024 * 1024 || totalBytes + stat.size > 64 * 1024 * 1024 || inputs.size >= 4096) {
      fail("SOURCE_TOO_LARGE", "Integration compiler input exceeds its file or project budget");
    }
    const real = fs.realpathSync(filename);
    const boundary = inside(root, filename) ? root : inside(sdkDirectory, filename) ? sdkDirectory : tsDirectory;
    if (!inside(boundary, real) || (boundary === root && real !== filename)) {
      fail("INVALID_IMPORT", "Function sources and installed packages cannot redirect imports through symlinks");
    }
    const bytes = fs.readFileSync(filename);
    totalBytes += bytes.length;
    if (bytes.length > 4 * 1024 * 1024 || totalBytes > 64 * 1024 * 1024 || inputs.size >= 4096) {
      fail("SOURCE_TOO_LARGE", "Integration compiler input exceeds its file or project budget");
    }
    let text;
    try { text = new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(bytes); }
    catch { fail("INVALID_SOURCE", "Integration compiler inputs must be UTF-8 text"); }
    inputs.set(filename, text);
    return text;
  };
  const json = (filename) => {
    try {
      const value = JSON.parse(read(filename));
      if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error("Expected an object");
      return value;
    }
    catch (error) {
      if (error instanceof BuildError) throw error;
      fail("INVALID_DEPENDENCIES", `Missing or invalid ${path.basename(filename)}`);
    }
  };
  const manifest = json(path.join(root, "package.json"));
  const lock = json(path.join(root, "package-lock.json"));
  if (lock.lockfileVersion !== 3 || !lock.packages?.[""] || manifest.workspaces) {
    fail("INVALID_DEPENDENCIES", "Use a standalone function project with an npm v3 lockfile");
  }
  for (const kind of ["dependencies", "devDependencies", "optionalDependencies"]) {
    if (JSON.stringify(sorted(manifest[kind])) !== JSON.stringify(sorted(lock.packages[""][kind]))) {
      fail("INVALID_DEPENDENCIES", "package.json and package-lock.json disagree; run npm ci before building");
    }
  }
  const checkedPackages = new Set();
  const allowedProjectFile = (filename) => {
    if (!inside(root, filename)) return false;
    const relative = path.relative(root, filename).split(path.sep).join("/");
    if (relative.startsWith("common/") || functions.some((name) => relative === `${name}.ts`)
        || relative === "package.json" || relative === "package-lock.json") return true;
    const parts = relative.split("/");
    const modules = parts.lastIndexOf("node_modules");
    const packageEnd = modules + (parts[modules + 1]?.startsWith("@") ? 3 : 2);
    if (modules < 0 || parts.length <= packageEnd) return false;
    const key = parts.slice(0, packageEnd).join("/");
    if (!checkedPackages.has(key)) {
      const pinned = lock.packages[key];
      if (!pinned || pinned.link || typeof pinned.version !== "string"
          || typeof pinned.integrity !== "string" || !/^sha(256|384|512)-/.test(pinned.integrity)
          || typeof pinned.resolved !== "string" || !pinned.resolved.startsWith("https://")) {
        fail("INVALID_DEPENDENCIES", `Package ${key} must have an exact registry version and integrity in the lockfile`);
      }
      const installed = json(path.join(root, key, "package.json"));
      if (installed.version !== pinned.version) fail("INVALID_DEPENDENCIES", `Installed ${key} differs from the lockfile; run npm ci`);
      checkedPackages.add(key);
    }
    return true;
  };
  const allowed = (filename) => allowedProjectFile(filename)
    || (inside(sdkDirectory, filename) && /\.(?:js|d\.ts)$/.test(filename))
    || (path.dirname(filename) === tsDirectory && /^lib\..*\.d\.ts$/.test(path.basename(filename)));
  const options = {
    target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext,
    moduleResolution: ts.ModuleResolutionKind.Bundler, lib: ["lib.es2022.d.ts"], types: [],
    strict: true, exactOptionalPropertyTypes: true, noUncheckedIndexedAccess: true,
    noEmit: true, skipLibCheck: false, allowJs: true, resolveJsonModule: true,
  };
  const host = ts.createCompilerHost(options);
  // TypeScript probes nonexistent candidates such as @scope/package.ts before
  // resolving @scope/package/index.d.ts. Only actual inputs have a package owner.
  host.readFile = (filename) => isFile(filename) && allowed(filename) ? read(filename) : undefined;
  host.fileExists = (filename) => isFile(filename) && allowed(filename);
  host.getSourceFile = (filename, version) => {
    const text = host.readFile(filename);
    return text === undefined ? undefined : ts.createSourceFile(filename, text, version, true);
  };
  host.resolveModuleNames = (names, containingFile) => names.map((name) => {
    if (name === "@meowlink/script" || name === HOST_MODULE) return {
      resolvedFileName: path.join(sdkDirectory, name === HOST_MODULE ? "host.d.ts" : "index.d.ts"),
      extension: ts.Extension.Dts, isExternalLibraryImport: true,
    };
    return ts.resolveModuleName(name, containingFile, options, host).resolvedModule;
  });
  const entries = [...functions].sort().map((name) => path.join(root, `${name}.ts`));
  for (const filename of entries) {
    const source = read(filename);
    if (!source || Buffer.byteLength(source) > 256 * 1024) fail("INVALID_SOURCE", "Each TypeScript entry must contain 1 to 256 KiB of UTF-8 text");
  }
  const program = ts.createProgram(entries, options, host);
  const diagnostics = ts.getPreEmitDiagnostics(program);
  if (diagnostics.length) throw new BuildError("TYPE_CHECK_FAILED", diagnostics.slice(0, 50).map((diagnostic) => {
    const result = { message: ts.flattenDiagnosticMessageText(diagnostic.messageText, "\n").slice(0, 4096), code: diagnostic.code };
    if (diagnostic.file) {
      const { line, character } = diagnostic.file.getLineAndCharacterOfPosition(diagnostic.start ?? 0);
      Object.assign(result, { file: path.relative(root, diagnostic.file.fileName), line: line + 1, column: character + 1 });
    }
    return result;
  }));
  const checker = program.getTypeChecker();
  for (const filename of entries) {
    const source = program.getSourceFile(filename);
    const module = source && checker.getSymbolAtLocation(source);
    const entry = module && checker.getExportsOfModule(module).find((symbol) => symbol.name === "default");
    if (!entry || !checker.getTypeOfSymbolAtLocation(entry, source).getCallSignatures().length) {
      fail("INVALID_ENTRY", `${path.basename(filename)} must export a default function`);
    }
  }
  signal?.throwIfAborted();
  let result;
  try {
    result = await esbuild.build({
      entryPoints: entries, absWorkingDir: root, outdir: "__script_build__", bundle: true,
      format: "esm", platform: "neutral", target: "es2022", mainFields: ["module", "main"],
      tsconfigRaw: {},
      write: false, metafile: true, logLevel: "silent", legalComments: "none",
      minifyWhitespace: true, keepNames: true,
      plugins: [{ name: "integration-inputs", setup(build) {
        build.onResolve({ filter: /.*/ }, async (args) => {
          if (args.pluginData?.resolved) return;
          if (args.path === HOST_MODULE) return { path: args.path, external: true, sideEffects: false };
          if (args.path === "@meowlink/script") return { path: sdkEntry };
          if (args.path.startsWith("node:")) return { errors: [{ text: "Node built-ins are unavailable in MeowCore scripts" }] };
          const resolved = await build.resolve(args.path, { importer: args.importer, resolveDir: args.resolveDir,
            kind: args.kind, pluginData: { resolved: true } });
          if (resolved.errors.length) return resolved;
          if (resolved.external || !allowed(resolved.path)) return { errors: [{ text: "Import is outside declared function sources or locked dependencies" }] };
          return resolved;
        });
        // Never fall through to esbuild's default filesystem loaders: every input
        // must be bounded, checked and included in the build's source identity.
        build.onLoad({ filter: /.*/ }, (args) => {
          const extension = path.extname(args.path);
          const loader = extension === ".json" ? "json" : /^\.[cm]?ts$/.test(extension) ? "ts"
            : /^\.[cm]?js$/.test(extension) ? "js" : undefined;
          if (!loader) return { errors: [{ text: `Unsupported Integration input extension: ${extension || "(none)"}` }] };
          const contents = allowed(args.path) ? read(args.path) : undefined;
          if (contents === undefined) return { errors: [{ text: "Undeclared Integration build input" }] };
          if (loader !== "json") checkImports(args.path, contents);
          return { contents, loader, resolveDir: path.dirname(args.path) };
        });
      } }],
    });
  } catch (error) {
    if (error instanceof BuildError) throw error;
    throw new BuildError("BUNDLE_FAILED", (error.errors ?? [{ text: "Integration bundling failed" }]).slice(0, 50)
      .map((entry) => ({ message: entry.text.slice(0, 4096) })));
  }
  if (result.warnings.length) fail("BUNDLE_FAILED", result.warnings[0].text.slice(0, 4096));
  signal?.throwIfAborted();
  // Hash the exact bytes consumed by both checking and bundling, including package
  // contents. Installed package versions alone are not evidence of identical bytes.
  const sourceInputs = [...inputs].filter(([name]) => inside(root, name))
    .map(([name, source]) => [path.relative(root, name).split(path.sep).join("/"), hash(source)])
    .sort(([a], [b]) => a.localeCompare(b));
  const build = { profile: "integration-es2022-v1", sdkVersion,
    sdkHash: hash([sdkEntry, path.join(sdkDirectory, "index.d.ts"), path.join(sdkDirectory, "host.d.ts")].map((name) => fs.readFileSync(name, "utf8")).join("\0")),
    lockHash: hash(read(path.join(root, "package-lock.json"))), typescript: ts.version, esbuild: esbuild.version };
  return result.outputFiles.map((output) => {
    const name = path.basename(output.path, ".js");
    const javascript = output.text;
    const size = Buffer.byteLength(javascript);
    if (!size || size > 4 * 1024 * 1024) fail("ARTIFACT_TOO_LARGE", "Bundled JavaScript exceeds 4 MiB");
    return { function: name, sourceHash: hash(JSON.stringify({ function: name, inputs: sourceInputs, build })),
      build, javascript, artifact: { format: "javascript-esm", sha256: hash(javascript), size, sdkVersion } };
  }).sort((a, b) => a.function.localeCompare(b.function));
}
