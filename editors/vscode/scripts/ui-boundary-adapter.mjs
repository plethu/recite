import {
  isCallMethod,
  isMemberCall,
  memberMethod,
  propertyName,
  staticMemberMethod,
  walkWithParents
} from "./ui-boundary-ast.mjs";

const MESSAGE_MODULE = "./messages.js";
const MESSAGE_WRAPPER = "clientMessage";
const OUTPUT_METHODS = new Set(["append", "appendLine", "createOutputChannel"]);
const MESSAGE_METHODS = new Set([
  "showWarningMessage",
  "showErrorMessage",
  "showInformationMessage"
]);
const VISIBLE_METHODS = new Set([...OUTPUT_METHODS, ...MESSAGE_METHODS]);
const GENERIC_INTERFACE_METHODS = new Set(["show", "showMessage", "write", "display"]);
const PASSTHROUGH_METHODS = new Map([
  ["serverStderr", "append"],
  ["serverNotification", "appendLine"]
]);

export function validateAdapter(ast, file, expected) {
  const imports = ast.body.filter((node) => node.type === "ImportDeclaration");
  const messageImports = imports.filter((node) => node.source.value === MESSAGE_MODULE);
  assert(messageImports.length === 1, `UI adapter must import ${MESSAGE_WRAPPER} exactly once (${file})`);
  const specifiers = messageImports[0].specifiers;
  assert(specifiers.length === 1 && specifiers[0].type === "ImportSpecifier" &&
    specifiers[0].imported.name === MESSAGE_WRAPPER &&
    specifiers[0].local.name === MESSAGE_WRAPPER,
  `UI adapter must use the canonical ${MESSAGE_WRAPPER} import (${file})`);
  assert(imports.every((node) => node.source.value === MESSAGE_MODULE),
    `UI adapter imports must remain limited to ${MESSAGE_MODULE} (${file})`);

  const createFunctions = ast.body.flatMap((node) => {
    const declaration = node.type === "ExportNamedDeclaration" ? node.declaration : node;
    return declaration?.type === "FunctionDeclaration" && declaration.id?.name === "createUserInterface"
      ? [declaration] : [];
  });
  assert(createFunctions.length === 1, `UI adapter must export createUserInterface (${file})`);
  const root = createFunctions[0];
  assert(root.params.length === 1 && root.params[0].type === "Identifier" &&
    root.params[0].name === "api", `UI adapter must accept only api (${file})`);

  const state = { outputDeclarations: [], messageCalls: [] };
  walkWithParents(root, (node, parent, ancestors) => {
    if (node.type === "Identifier" && node.name === MESSAGE_WRAPPER) {
      const imported = parent?.type === "ImportSpecifier";
      const directCallee = parent?.type === "CallExpression" && parent.callee === node;
      assert(imported || directCallee, `UI adapter must not alias or expose ${MESSAGE_WRAPPER} (${file})`);
    }
    if (node.type === "Identifier" && node.name === "output") {
      const declaration = parent?.type === "VariableDeclarator" && parent.id === node;
      const receiver = parent?.type === "MemberExpression" && parent.object === node;
      assert(declaration || receiver, `UI adapter must not expose its output channel (${file})`);
    }
    if (node.type === "VariableDeclarator") validateDeclaration(node, file, state);
    if (node.type === "AssignmentExpression" || node.type === "UpdateExpression") {
      throw new Error(`UI adapter forbids reassignment (${file})`);
    }
    if (["ConditionalExpression", "TemplateLiteral", "BinaryExpression", "SequenceExpression",
      "SpreadElement", "RestElement"].includes(node.type)) {
      throw new Error(`UI adapter uses an unsupported expression shape: ${node.type} (${file})`);
    }
    if (node.type === "FunctionDeclaration" && node !== root ||
        node.type === "FunctionExpression" || node.type === "ArrowFunctionExpression") {
      for (const parameter of node.params) {
        assert(parameter.type === "Identifier", `UI adapter parameters must be simple identifiers (${file})`);
        assert(!["api", MESSAGE_WRAPPER, "output"].includes(parameter.name),
          `UI adapter shadows a reserved binding: ${parameter.name} (${file})`);
      }
    }
    if (node.type === "CatchClause" && node.param?.type === "Identifier") {
      assert(!["api", MESSAGE_WRAPPER, "output"].includes(node.param.name),
        `UI adapter shadows a reserved binding: ${node.param.name} (${file})`);
    }
    if (node.type === "MemberExpression" && node.computed &&
        (isWindowReceiver(node.object) || isVisibleMember(node.object) ||
          node.object.type === "Identifier" && node.object.name === "output")) {
      throw new Error(`UI adapter forbids computed visible sink access (${file})`);
    }
    if (node.type === "Property") {
      const name = propertyName(node);
      assert(!name || !GENERIC_INTERFACE_METHODS.has(name),
        `UI adapter method must be semantic, not generic: ${name} (${file})`);
    }
    if (node.type === "CallExpression") validateCall(node, parent, ancestors, file, state);
    if (node.type === "NewExpression") {
      assert(node.callee.type === "Identifier" && node.callee.name === "Error" &&
        node.arguments.length === 1 && node.arguments[0].type === "CallExpression" &&
        node.arguments[0].callee.type === "Identifier" &&
        node.arguments[0].callee.name === MESSAGE_WRAPPER,
      `UI adapter errors must use a direct canonical projection (${file})`);
    }
  });

  assert(state.outputDeclarations.length === 1,
    `UI adapter must create one private output channel (${file})`);
  assert(state.messageCalls.length === expected.size,
    `UI adapter message projections must cover exactly the registered IDs (${file})`);
  const used = new Set(state.messageCalls.map((call) => call.arguments[1].value));
  assert(used.size === expected.size && [...expected].every((id) => used.has(id)),
    `UI adapter message projections must cover exactly the registered IDs (${file})`);
}

function validateDeclaration(node, file, state) {
  assert(node.id.type === "Identifier", `UI adapter declarations must not destructure values (${file})`);
  assert(node.id.name !== MESSAGE_WRAPPER, `UI adapter must not redeclare ${MESSAGE_WRAPPER} (${file})`);
  if (node.id.name === "api") throw new Error(`UI adapter shadows api (${file})`);
  if (node.id.name === "output") {
    assert(node.init?.type === "CallExpression" && isMemberCall(node.init, "api", "window",
      "createOutputChannel"), `UI adapter output must be created directly (${file})`);
    state.outputDeclarations.push(node);
  }
  if (node.init?.type === "Identifier" && [MESSAGE_WRAPPER, "output"].includes(node.init.name)) {
    throw new Error(`UI adapter forbids aliases of ${node.init.name} (${file})`);
  }
  if (isVisibleMember(node.init)) {
    throw new Error(`UI adapter forbids aliases of visible host sinks (${file})`);
  }
}

function validateCall(node, parent, ancestors, file, state) {
  if (node.callee.type === "MemberExpression" && node.callee.optional) {
    throw new Error(`UI adapter forbids optional visible sink access (${file})`);
  }
  if (isCallMethod(node, "call") || isCallMethod(node, "apply") || isCallMethod(node, "bind")) {
    throw new Error(`UI adapter forbids call/apply/bind sink aliases (${file})`);
  }
  if (node.callee.type === "Identifier" && node.callee.name === MESSAGE_WRAPPER) {
    assert(!node.optional, `UI adapter projections must call the canonical wrapper directly (${file})`);
    assertMessageCall(node, file, state);
    const directParent = parent?.type === "CallExpression" &&
      parent.arguments.some((argument) => argument === node);
    const errorParent = parent?.type === "NewExpression" && parent.callee.type === "Identifier" &&
      parent.callee.name === "Error" && parent.arguments[0] === node;
    assert(directParent || errorParent, `UI adapter projections must be used directly (${file})`);
    return;
  }
  const method = memberMethod(node.callee);
  if (!method || !VISIBLE_METHODS.has(method)) return;
  if (method === "createOutputChannel") {
    assert(isMemberCall(node, "api", "window", method) && node.arguments.length === 1 &&
      isCanonicalCall(node.arguments[0]), `UI output creation must use a direct projection (${file})`);
    return;
  }
  if (isOutputMember(node.callee)) {
    assert(node.arguments.length === 1, `UI output calls must have one argument (${file})`);
    if (isCanonicalCall(node.arguments[0])) return;
    const functionNode = ancestors.findLast((ancestor) =>
      ancestor.type === "FunctionExpression" || ancestor.type === "ArrowFunctionExpression" ||
      ancestor.type === "FunctionDeclaration");
    const methodNode = functionNode && ancestors
      .slice(0, ancestors.indexOf(functionNode)).findLast((ancestor) => ancestor.type === "Property");
    const property = methodNode && propertyName(methodNode);
    const parameter = functionNode?.params.find((candidate) =>
      candidate.type === "Identifier" && candidate.name === node.arguments[0]?.name);
    assert(parameter && PASSTHROUGH_METHODS.get(property) === method,
      `UI output calls must use a direct projection or approved server payload (${file})`);
    return;
  }
  if (MESSAGE_METHODS.has(method)) {
    assert(isMemberCall(node, "api", "window", method) && node.arguments.length === 1 &&
      isCanonicalCall(node.arguments[0]), `UI message calls must use a direct projection (${file})`);
    return;
  }
  throw new Error(`UI adapter uses an unsupported visible sink (${file})`);
}

function assertMessageCall(node, file, state) {
  assert(node.arguments.length >= 2 && node.arguments.length <= 3 &&
    node.arguments[0].type === "Identifier" && node.arguments[0].name === "api",
  `canonical projections must receive api directly (${file})`);
  const id = node.arguments[1];
  assert(id.type === "Literal" && typeof id.value === "string" && /^[a-z0-9-]+$/.test(id.value) &&
    (id.raw === JSON.stringify(id.value) || id.raw === `'${id.value}'`),
  `canonical projections must use an unescaped literal ID (${file})`);
  if (node.arguments[2]) assert(isSimpleValue(node.arguments[2]),
    `canonical projection details must be direct values (${file})`);
  state.messageCalls.push(node);
}

function isOutputMember(node) {
  return node?.type === "MemberExpression" && !node.computed &&
    node.object.type === "Identifier" && node.object.name === "output" &&
    (node.property.name === "append" || node.property.name === "appendLine");
}

function isCanonicalCall(node) {
  return node?.type === "CallExpression" && node.callee.type === "Identifier" &&
    node.callee.name === MESSAGE_WRAPPER;
}

function isVisibleMember(node) {
  if (node?.type !== "MemberExpression") return false;
  return VISIBLE_METHODS.has(memberMethod(node) ?? staticMemberMethod(node));
}

function isWindowReceiver(node) {
  return node?.type === "MemberExpression" && !node.computed && node.property.type === "Identifier" &&
    node.property.name === "window";
}

function isSimpleValue(node) {
  return node?.type === "Identifier" || node?.type === "MemberExpression" && !node.computed;
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}
