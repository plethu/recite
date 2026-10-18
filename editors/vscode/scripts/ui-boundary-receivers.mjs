import {
  memberMethod,
  propertyName,
  staticMemberMethod,
  walkWithParents,
} from "./ui-boundary-ast.mjs";

const MESSAGE_MODULE = "./messages.ts";
const ADAPTER_MODULE = "./user-interface.ts";
const MESSAGE_WRAPPER = "clientMessage";
const ADAPTER_FILE = "user-interface.ts";
const OUTPUT_METHODS = new Set(["append", "appendLine", "createOutputChannel"]);
const MESSAGE_METHODS = new Set([
  "showWarningMessage",
  "showErrorMessage",
  "showInformationMessage",
]);
const VISIBLE_METHODS = new Set([...OUTPUT_METHODS, ...MESSAGE_METHODS]);
const UI_NAMES = new Set(["ui", "userInterface"]);
const API_NAMES = new Set(["api", "vscode"]);
const CONTROLLER_NAMES = new Set(["controller"]);

export function collectUiNames(ast) {
  const names = new Set();
  walkWithParents(ast, (node) => {
    if (
      node.type === "VariableDeclarator" &&
      node.id.type === "Identifier" &&
      isCreateUserInterfaceCall(node.init)
    )
      names.add(node.id.name);
  });
  return names;
}

export function isValidUiCall(node, contracts, uiNames) {
  const callee = unwrap(node.callee);
  if (callee?.type !== "MemberExpression" || callee.computed || callee.optional || node.optional) {
    return false;
  }
  if (!isKnownUiExpression(callee.object, uiNames)) return false;
  const method = memberMethod(callee);
  const contract = method && contracts[method];
  if (!contract) return false;
  const expectsPayload =
    ((contract.kind === "projection" || contract.kind === "visible-projection") &&
      contract.argument) ||
    contract.kind === "passthrough" ||
    contract.kind === "host-passthrough" ||
    (contract.kind === "host-capability-projection" && contract.argument) ||
    (contract.kind === "host-input-projection" && contract.argument) ||
    contract.kind === "host-membership";
  assert(
    node.arguments.length === (expectsPayload ? 1 : 0) &&
      (!expectsPayload || isDirectPayload(node.arguments[0])),
    `UI method ${method} has an invalid direct contract payload outside ${ADAPTER_FILE}`,
  );
  return true;
}

export function isAllowedUiCallMember(node, parent, contracts, uiNames) {
  return (
    parent?.type === "CallExpression" &&
    parent.callee === node &&
    isValidUiCall(parent, contracts, uiNames)
  );
}

export function isCapabilityMember(node, contracts, uiNames) {
  const current = unwrap(node);
  if (current?.type !== "MemberExpression") return false;
  if (isDynamicKnownAccess(current, uiNames)) return true;
  const method = staticMethod(current);
  return (
    (isKnownUiExpression(current.object, uiNames) &&
      (!method || !Object.hasOwn(contracts, method))) ||
    isVisibleSinkMember(current, uiNames)
  );
}

export function isCapabilityPattern(pattern, source, uiNames) {
  if (!isDestructuringPattern(pattern)) return false;
  if (isCapabilityExpression(source, uiNames)) return true;
  const value = unwrap(source);
  if (value?.type === "Identifier" && API_NAMES.has(value.name)) {
    return patternContainsProperty(pattern, "window");
  }
  if (
    value?.type === "Identifier" &&
    ["messages", "messageModule", "messagesModule"].includes(value.name)
  ) {
    return patternContainsProperty(pattern, MESSAGE_WRAPPER);
  }
  return false;
}

export function isUiInjectionAssignment(left, right) {
  const target = unwrap(left);
  return (
    target?.type === "MemberExpression" &&
    !target.computed &&
    memberMethod(target) === "userInterface" &&
    (target.object.type === "ThisExpression" ||
      (target.object.type === "Identifier" && CONTROLLER_NAMES.has(target.object.name))) &&
    unwrap(right)?.type === "Identifier" &&
    UI_NAMES.has(unwrap(right).name)
  );
}

function isDestructuringPattern(node) {
  return node?.type === "ObjectPattern" || node?.type === "ArrayPattern";
}

function patternContainsProperty(pattern, name) {
  if (pattern?.type === "ObjectPattern") {
    return pattern.properties.some(
      (property) =>
        (property.type === "Property" &&
          (propertyName(property) === name || patternContainsProperty(property.value, name))) ||
        (property.type === "RestElement" && patternContainsProperty(property.argument, name)),
    );
  }
  if (pattern?.type === "ArrayPattern") {
    return pattern.elements.some((element) => patternContainsProperty(element, name));
  }
  if (pattern?.type === "AssignmentPattern") return patternContainsProperty(pattern.left, name);
  return false;
}

export function isVisibleSinkMember(node, _uiNames) {
  const current = unwrap(node);
  if (current?.type !== "MemberExpression") return false;
  const method = staticMethod(current);
  if (!method || !VISIBLE_METHODS.has(method)) return false;
  if (MESSAGE_METHODS.has(method) || method === "createOutputChannel") {
    return isKnownWindowExpression(current.object);
  }
  return isKnownOutputExpression(current.object) || isKnownWindowExpression(current.object);
}

export function isDynamicKnownAccess(node, uiNames) {
  const current = unwrap(node);
  return (
    current?.type === "MemberExpression" &&
    current.computed &&
    (isKnownWindowExpression(current.object) ||
      isKnownUiExpression(current.object, uiNames) ||
      isKnownOutputExpression(current.object))
  );
}

export function isKnownUiMember(node, uiNames) {
  const current = unwrap(node);
  return (
    current?.type === "MemberExpression" &&
    !current.computed &&
    isKnownUiExpression(current.object, uiNames)
  );
}

function isKnownUiExpression(node, uiNames) {
  const current = unwrap(node);
  if (current?.type === "Identifier") {
    return uiNames.has(current.name) || UI_NAMES.has(current.name);
  }
  if (current?.type === "CallExpression") return isCreateUserInterfaceCall(current);
  if (current?.type !== "MemberExpression" || current.computed || current.optional) return false;
  if (memberMethod(current) === "userInterface") {
    const root = rootOf(current.object);
    return (
      current.object.type === "ThisExpression" ||
      root?.type === "ThisExpression" ||
      (root?.type === "Identifier" && CONTROLLER_NAMES.has(root.name))
    );
  }
  return isKnownUiExpression(current.object, uiNames);
}

function isKnownOutputExpression(node) {
  const current = unwrap(node);
  if (current?.type === "Identifier") {
    return ["output", "channel", "outputChannel"].includes(current.name);
  }
  if (isCreateOutputChannelCall(current)) return true;
  if (current?.type !== "MemberExpression" || current.computed || current.optional) return false;
  const root = rootOf(current);
  return (
    (root?.type === "ThisExpression" &&
      ["output", "channel", "outputChannel"].includes(memberMethod(current))) ||
    (root?.type === "Identifier" &&
      CONTROLLER_NAMES.has(root.name) &&
      ["output", "channel", "outputChannel"].includes(memberMethod(current)))
  );
}

function isKnownWindowExpression(node) {
  const current = unwrap(node);
  if (current?.type === "Identifier" && current.name === "window") return true;
  if (current?.type !== "MemberExpression" || current.computed) return false;
  if (memberMethod(current) === "window") {
    const root = rootOf(current.object);
    return root?.type === "Identifier" && API_NAMES.has(root.name);
  }
  return isKnownWindowExpression(current.object);
}

function isCreateOutputChannelCall(node) {
  const current = unwrap(node);
  return (
    current?.type === "CallExpression" &&
    !current.optional &&
    isMemberNamed(current.callee, "createOutputChannel") &&
    isKnownWindowExpression(current.callee.object)
  );
}

export function isCreateUserInterfaceCall(node) {
  const current = unwrap(node);
  return (
    current?.type === "CallExpression" &&
    !current.optional &&
    current.callee.type === "Identifier" &&
    current.callee.name === "createUserInterface"
  );
}

export function isCapabilityExpression(node, uiNames) {
  const current = unwrap(node);
  if (!current) return false;
  if (current.type === "Identifier") {
    return (
      current.name === MESSAGE_WRAPPER ||
      isKnownUiExpression(current, uiNames) ||
      isKnownOutputExpression(current) ||
      isKnownWindowExpression(current) ||
      ["messages", "messageModule", "messagesModule"].includes(current.name)
    );
  }
  if (isCreateUserInterfaceCall(current) || isCreateOutputChannelCall(current)) return true;
  if (
    isMessageExpression(current) ||
    isKnownWindowExpression(current) ||
    isKnownUiExpression(current, uiNames) ||
    isKnownOutputExpression(current)
  )
    return true;
  return false;
}

export function isMessageExpression(node) {
  const current = unwrap(node);
  if (current?.type === "Identifier") return current.name === MESSAGE_WRAPPER;
  if (current?.type !== "MemberExpression") return false;
  const method = staticMethod(current);
  if (method !== MESSAGE_WRAPPER) return false;
  const root = rootOf(current.object);
  return (
    root?.type === "Identifier" &&
    ["messages", "messageModule", "messagesModule"].includes(root.name)
  );
}

export function isMessageMember(node) {
  return isMessageExpression(node);
}

export function isMessageCapabilityMember(node) {
  const current = unwrap(node);
  return current?.type === "MemberExpression" && isMessageExpression(current.object);
}

export function isMessageIdentifier(node) {
  const current = unwrap(node);
  return current?.type === "Identifier" && current.name === MESSAGE_WRAPPER;
}

export function isMessageCallName(node) {
  const current = unwrap(node);
  return (
    (current?.type === "Identifier" && current.name === MESSAGE_WRAPPER) ||
    isMessageExpression(current)
  );
}

export function isRequireOfProtectedModule(node) {
  const callee = unwrap(node.callee);
  return (
    callee?.type === "Identifier" &&
    callee.name === "require" &&
    node.arguments.length === 1 &&
    node.arguments[0].type === "Literal" &&
    isProtectedModule(node.arguments[0].value, MESSAGE_MODULE, ADAPTER_MODULE)
  );
}

export function isDirectPayload(node) {
  const current = unwrap(node);
  if (current?.type === "Identifier" || current?.type === "ThisExpression") return true;
  return (
    current?.type === "MemberExpression" &&
    !current.computed &&
    !current.optional &&
    current.property.type === "Identifier" &&
    isDirectPayload(current.object)
  );
}

export function staticMethod(node) {
  const current = unwrap(node);
  return memberMethod(current) ?? staticMemberMethod(current);
}

function isMemberNamed(node, name) {
  return staticMethod(node) === name && unwrap(node)?.type === "MemberExpression";
}

function unwrap(node) {
  return node?.type === "ChainExpression" ? node.expression : node;
}

function rootOf(node) {
  const current = unwrap(node);
  if (current?.type === "Identifier" || current?.type === "ThisExpression") return current;
  if (current?.type === "MemberExpression") return rootOf(current.object);
  return undefined;
}

export function isProtectedModule(value, ...modules) {
  if (typeof value !== "string") return false;
  // Bun and TypeScript resolve emitted .js specifiers to .ts sources, and Bun also
  // accepts extensionless imports. Every spelling must retain the same boundary.
  const stem = (specifier) => specifier.replace(/\.[cm]?[jt]sx?$/, "");
  const imported = stem(value);
  return modules.some((module) => {
    const protectedStem = stem(module);
    return imported === protectedStem || imported.endsWith(`/${protectedStem.slice(2)}`);
  });
}
export { MESSAGE_MODULE, ADAPTER_MODULE, MESSAGE_WRAPPER, ADAPTER_FILE, VISIBLE_METHODS };

function assert(condition, message) {
  if (!condition) throw new Error(message);
}
