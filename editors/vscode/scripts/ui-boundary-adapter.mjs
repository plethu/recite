import {
  isMemberCall,
  memberMethod,
  propertyName
} from "./ui-boundary-ast.mjs";

const MESSAGE_MODULE = "./messages.js";
const MESSAGE_WRAPPER = "clientMessage";

// This is the UI service contract. The adapter validator and the outside
// policy share it instead of trying to infer arbitrary program dataflow.
export const UI_METHOD_CONTRACTS = Object.freeze({
  serverStartFailed: { kind: "projection", id: "lsp-client-start-failed", argument: "detail" },
  serverError: { kind: "projection", id: "lsp-client-error", argument: "detail" },
  serverExited: { kind: "projection", id: "lsp-client-exited", argument: "detail" },
  restartScheduled: { kind: "projection", id: "lsp-client-restart-scheduled", argument: "detail" },
  restartExhausted: { kind: "projection", id: "lsp-client-restart-exhausted" },
  actionStale: { kind: "projection", id: "lsp-client-action-stale" },
  actionClosed: { kind: "projection", id: "lsp-client-action-closed" },
  actionReopened: { kind: "projection", id: "lsp-client-action-reopened" },
  actionExpired: { kind: "projection", id: "lsp-client-action-expired" },
  actionEvicted: { kind: "projection", id: "lsp-client-action-evicted" },
  actionApplyFailed: { kind: "projection", id: "lsp-client-action-apply-failed" },
  actionUnknown: { kind: "projection", id: "lsp-client-action-unknown" },
  configurationPathInvalid: { kind: "error", id: "lsp-client-config-path-invalid" },
  configurationArgsInvalid: { kind: "error", id: "lsp-client-config-args-invalid" },
  configurationProjectRootInvalid: { kind: "error", id: "lsp-client-config-project-root-invalid" },
  configurationProjectRootNeedsWorkspace: {
    kind: "error", id: "lsp-client-config-project-root-needs-workspace"
  },
  serverNotRunning: { kind: "error", id: "lsp-client-not-running" },
  serverStderr: { kind: "passthrough", sink: "append" },
  serverNotification: { kind: "passthrough", sink: "appendLine" },
  dispose: { kind: "dispose" }
});

export function validateAdapter(ast, file, expected) {
  assert(ast.body.length === 2 && ast.body[0].type === "ImportDeclaration" &&
    ast.body[1].type === "ExportNamedDeclaration",
  `UI adapter must contain only its canonical import and factory (${file})`);
  const imports = ast.body.filter((node) => node.type === "ImportDeclaration");
  const exports = ast.body.filter((node) => node.type === "ExportNamedDeclaration");
  assert(imports.length === 1 && exports.length === 1,
    `UI adapter must contain one import and one export (${file})`);
  const messageImport = imports[0];
  assert(messageImport.source.value === MESSAGE_MODULE && messageImport.specifiers.length === 1 &&
    (messageImport.attributes?.length ?? 0) === 0 &&
    messageImport.specifiers[0].type === "ImportSpecifier" &&
    messageImport.specifiers[0].imported.name === MESSAGE_WRAPPER &&
    messageImport.specifiers[0].local.name === MESSAGE_WRAPPER,
  `UI adapter must use the canonical ${MESSAGE_WRAPPER} import (${file})`);
  const declaration = exports[0].declaration;
  assert(exports[0].specifiers.length === 0 && exports[0].source === null &&
    declaration?.type === "FunctionDeclaration" && declaration.id?.name === "createUserInterface" &&
    !declaration.async && !declaration.generator,
  `UI adapter must export only createUserInterface (${file})`);
  const factory = declaration;
  assert(factory.params.length === 1 && factory.params[0].type === "Identifier" &&
    factory.params[0].name === "api" && factory.body.body.length === 2,
  `UI adapter factory must accept only api and have an exact body (${file})`);

  const outputStatement = factory.body.body[0];
  assert(outputStatement.type === "VariableDeclaration" && outputStatement.kind === "const" &&
    outputStatement.declarations.length === 1 && outputStatement.declarations[0].id.type === "Identifier" &&
    outputStatement.declarations[0].id.name === "output",
  `UI factory must declare one private output channel (${file})`);
  const output = outputStatement.declarations[0];
  assert(isMemberCall(output.init, "api", "window", "createOutputChannel") &&
    output.init.arguments.length === 1 && isCanonicalCall(output.init.arguments[0]),
  `UI output must be created directly from the canonical display projection (${file})`);
  assertMessageCall(output.init.arguments[0], file, "lsp-client-display-name");

  const returnStatement = factory.body.body[1];
  assert(returnStatement.type === "ReturnStatement" && isObjectFreeze(returnStatement.argument),
    `UI factory must return Object.freeze of its semantic service (${file})`);
  const service = returnStatement.argument.arguments[0];
  assert(service.properties.length === Object.keys(UI_METHOD_CONTRACTS).length,
    `UI service methods must match the declared contract (${file})`);
  const seen = new Set();
  for (const property of service.properties) {
    const name = propertyName(property);
    assert(property.type === "Property" && property.method === true && name &&
      Object.hasOwn(UI_METHOD_CONTRACTS, name) && !seen.has(name),
    `UI service contains an unsupported or duplicate method: ${name ?? "computed"} (${file})`);
    seen.add(name);
    validateMethod(property, UI_METHOD_CONTRACTS[name], file);
  }
  assert(seen.size === Object.keys(UI_METHOD_CONTRACTS).length,
    `UI service methods must match the declared contract (${file})`);

  const ids = ["lsp-client-display-name", ...Object.values(UI_METHOD_CONTRACTS)
    .filter((contract) => contract.id).map((contract) => contract.id)];
  assert(ids.length === expected.size && ids.every((id) => expected.has(id)),
    `UI adapter contract IDs must match registered message IDs (${file})`);
  return UI_METHOD_CONTRACTS;
}

function validateMethod(property, contract, file) {
  const method = property.value;
  assert(method.type === "FunctionExpression" && !method.async && !method.generator &&
    method.params.every((param) => param.type === "Identifier"),
    `UI adapter method parameters must be simple identifiers (${file})`);
  if (contract.kind === "projection") {
    assert(method.params.length === (contract.argument ? 1 : 0),
      `UI method ${propertyName(property)} has the wrong argument count (${file})`);
    const statement = oneStatement(method, file, propertyName(property));
    assert(statement.type === "ExpressionStatement" && isOutputCall(statement.expression, "appendLine"),
      `UI method ${propertyName(property)} must append a projection directly (${file})`);
    const projection = statement.expression.arguments[0];
    assert(isCanonicalCall(projection),
      `UI method ${propertyName(property)} must append a projection directly (${file})`);
    assertMessageCall(projection, file, contract.id, contract.argument);
    return;
  }
  if (contract.kind === "error") {
    assert(method.params.length === 0,
      `UI adapter method ${propertyName(property)} has the wrong argument count (${file})`);
    const statement = oneStatement(method, file, propertyName(property));
    const error = statement.type === "ReturnStatement" ? statement.argument : undefined;
    assert(error?.type === "NewExpression" && error.callee.type === "Identifier" &&
      error.callee.name === "Error" && error.arguments.length === 1 &&
      isCanonicalCall(error.arguments[0]),
    `UI method ${propertyName(property)} must return a canonical Error (${file})`);
    assertMessageCall(error.arguments[0], file, contract.id);
    return;
  }
  if (contract.kind === "passthrough") {
    assert(method.params.length === 1,
      `UI adapter method ${propertyName(property)} has the wrong argument count (${file})`);
    const statement = oneStatement(method, file, propertyName(property));
    assert(statement.type === "ExpressionStatement" && isOutputCall(statement.expression, contract.sink),
      `UI adapter method ${propertyName(property)} must pass one payload directly (${file})`);
    const payload = statement.expression.arguments[0];
    assert(payload.type === "Identifier" && payload.name === method.params[0].name,
      `UI adapter method ${propertyName(property)} must pass its payload directly (${file})`);
    return;
  }
  assert(method.params.length === 0,
    `UI adapter method ${propertyName(property)} has the wrong argument count (${file})`);
  const statement = oneStatement(method, file, propertyName(property));
  assert(statement.type === "ExpressionStatement" && isOutputCall(statement.expression, "dispose"),
    `UI method ${propertyName(property)} must dispose the private output directly (${file})`);
  assert(statement.expression.arguments.length === 0,
    `UI method ${propertyName(property)} must not pass disposal arguments (${file})`);
}

function assertMessageCall(node, file, id, argument) {
  assert(node.arguments.length === (argument ? 3 : 2) &&
    node.arguments[0].type === "Identifier" && node.arguments[0].name === "api",
  `canonical projection ${id} must receive api directly (${file})`);
  const literal = node.arguments[1];
  assert(literal.type === "Literal" && typeof literal.value === "string" &&
    /^[a-z0-9-]+$/.test(literal.value) &&
    (literal.raw === JSON.stringify(literal.value) || literal.raw === `'${literal.value}'`) &&
    literal.value === id,
  `canonical projection must use the registered unescaped ID ${id} (${file})`);
  if (argument) {
    assert(node.arguments[2].type === "Identifier" && node.arguments[2].name === argument,
      `canonical projection ${id} must receive its direct detail (${file})`);
  }
}

function isOutputCall(node, method) {
  return node?.type === "CallExpression" && node.arguments.length === 1 - (method === "dispose") &&
    node.callee.type === "MemberExpression" && !node.callee.computed && !node.callee.optional &&
    node.callee.object.type === "Identifier" && node.callee.object.name === "output" &&
    memberMethod(node.callee) === method;
}

function isCanonicalCall(node) {
  return node?.type === "CallExpression" && node.callee.type === "Identifier" &&
    node.callee.name === MESSAGE_WRAPPER && !node.optional;
}

function isObjectFreeze(node) {
  return node?.type === "CallExpression" && node.arguments.length === 1 &&
    node.callee.type === "MemberExpression" && !node.callee.computed && !node.callee.optional &&
    node.callee.object.type === "Identifier" && node.callee.object.name === "Object" &&
    memberMethod(node.callee) === "freeze" && node.arguments[0].type === "ObjectExpression" &&
    node.arguments[0].properties.every((property) => property.type === "Property" && !property.computed);
}

function oneStatement(method, file, name) {
  assert(method.body.type === "BlockStatement" && method.body.body.length === 1,
    `UI adapter method ${name} must have one exact statement (${file})`);
  return method.body.body[0];
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}
