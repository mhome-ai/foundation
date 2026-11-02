import ts from "typescript";

const IDENTIFIER = /^[A-Za-z_][A-Za-z0-9_]*$/;
const SCALARS = new Set(["bool", "number", "string", "timestamp", "duration", "temperature", "image", "object", "value"]);

export class BuildError extends Error {
  constructor(code, diagnostics) {
    super(diagnostics.map((d) => d.message).join("\n"));
    this.name = "BuildError";
    this.code = code;
    this.diagnostics = diagnostics;
  }
}

/** Read only the authoring declaration. Never evaluate source to infer types. */
export function readContract(sourceFile, sdkVersion) {
  const fail = (node, message) => {
    const { line, character } = sourceFile.getLineAndCharacterOfPosition(node.getStart(sourceFile));
    throw new BuildError("INVALID_CONTRACT", [{ file: sourceFile.fileName.split(/[\\/]/).at(-1), line: line + 1, column: character + 1, message }]);
  };
  const imported = new Map();
  for (const statement of sourceFile.statements) {
    if (!ts.isImportDeclaration(statement) || statement.moduleSpecifier.text !== "@meowlink/script"
        || statement.importClause?.isTypeOnly) continue;
    const bindings = statement.importClause?.namedBindings;
    if (bindings && ts.isNamedImports(bindings)) {
      for (const binding of bindings.elements) {
        if (!binding.isTypeOnly) imported.set(binding.name.text, binding.propertyName?.text ?? binding.name.text);
      }
    }
  }
  const defaults = sourceFile.statements.filter((s) => ts.isExportAssignment(s) && !s.isExportEquals);
  if (defaults.length !== 1) fail(sourceFile, "Export one default defineScript({ input, output, run }) declaration");
  const call = defaults[0].expression;
  if (!ts.isCallExpression(call) || !ts.isIdentifier(call.expression)
      || imported.get(call.expression.text) !== "defineScript" || call.arguments.length !== 1
      || !ts.isObjectLiteralExpression(call.arguments[0])) {
    fail(call, "Default export must directly call defineScript imported from @meowlink/script");
  }
  const definition = new Map();
  for (const property of call.arguments[0].properties) {
    if ((!ts.isPropertyAssignment(property) && !ts.isMethodDeclaration(property))
        || !property.name || ts.isComputedPropertyName(property.name)) {
      fail(property, "Script declarations use explicit input, output and run properties");
    }
    const key = property.name.text;
    if (!["input", "output", "run"].includes(key) || definition.has(key)) fail(property, "Unknown or duplicate script declaration property");
    definition.set(key, property);
  }
  if (definition.size !== 3) fail(call, "Script declaration requires input, output and run");

  const descriptor = (node, depth = 0) => {
    if (depth > 64) fail(node, "Contract list nesting exceeds 64 levels");
    if (ts.isPropertyAccessExpression(node) && ts.isIdentifier(node.expression)
        && imported.get(node.expression.text) === "t" && SCALARS.has(node.name.text)) return node.name.text;
    if (ts.isCallExpression(node) && ts.isPropertyAccessExpression(node.expression)
        && ts.isIdentifier(node.expression.expression) && imported.get(node.expression.expression.text) === "t"
        && node.expression.name.text === "list" && node.arguments.length === 1) {
      return { list: descriptor(node.arguments[0], depth + 1) };
    }
    fail(node, "Use a literal SDK descriptor such as t.number or t.list(t.number)");
  };
  const fields = (property) => {
    if (!ts.isPropertyAssignment(property) || !ts.isObjectLiteralExpression(property.initializer)) {
      fail(property, "Input/output must be literal named-field objects");
    }
    const names = new Set();
    const result = [];
    for (const field of property.initializer.properties) {
      if (!ts.isPropertyAssignment(field) || ts.isComputedPropertyName(field.name)) fail(field, "Contract fields cannot use spreads, methods, shorthand or computed names");
      const name = field.name.text;
      if (typeof name !== "string" || name.length > 255 || !IDENTIFIER.test(name) || names.has(name)) fail(field, "Contract fields must have unique ASCII identifier names");
      names.add(name);
      result.push({ name, type: descriptor(field.initializer) });
    }
    if (result.length > 65_536) fail(property, "Contract has too many fields");
    return result;
  };
  return { version: 1, sdkVersion, input: fields(definition.get("input")), output: fields(definition.get("output")) };
}
