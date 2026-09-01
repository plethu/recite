import {
  isCallMethod,
  memberMethod,
  parseSource,
  receiverRoot,
  staticMemberMethod,
  walkWithParents
} from "./ui-boundary-ast.mjs";
import { validateAdapter } from "./ui-boundary-adapter.mjs";

const MESSAGE_MODULE = "./messages.js";
const MESSAGE_WRAPPER = "clientMessage";
const ADAPTER_FILE = "user-interface.js";
const OUTPUT_METHODS = new Set(["append", "appendLine", "createOutputChannel"]);
const MESSAGE_METHODS = new Set([
  "showWarningMessage",
  "showErrorMessage",
  "showInformationMessage"
]);
const VISIBLE_METHODS = new Set([...OUTPUT_METHODS, ...MESSAGE_METHODS]);

/**
 * Enforce the small source/UI boundary used by the VS Code package.
 *
 * This is deliberately a structural policy, not a source-level dataflow or
 * name-resolution implementation. The output channel is created and closed
 * over by one adapter, so resolving aliases in the rest of the extension
 * would add complexity without improving the boundary.
 */
export function assertUiBoundary(sourceFiles, ownedIds, projectedMessages, adapterFile = ADAPTER_FILE) {
  const entries = new Map(sourceFiles);
  const adapterSource = entries.get(adapterFile);
  assert(typeof adapterSource === "string", `UI adapter ${adapterFile} is required`);
  const expected = new Set(ownedIds);
  for (const id of expected) {
    assert(Object.hasOwn(projectedMessages, id), `UI message projection is missing ${id}`);
  }

  validateAdapter(parseSource(adapterSource, adapterFile), adapterFile, expected);
  for (const [file, source] of entries) {
    if (file === adapterFile || file === "messages.js") continue;
    validateOutside(parseSource(source, file), file);
  }
}

function validateOutside(ast, file) {
  walkWithParents(ast, (node) => {
    if (node.type === "ImportDeclaration" && node.source.value === MESSAGE_MODULE) {
      throw new Error(`only ${ADAPTER_FILE} may import ${MESSAGE_WRAPPER} (${file})`);
    }
    if (node.type === "CallExpression") {
      if (isMessageCallName(node.callee)) {
        throw new Error(`only ${ADAPTER_FILE} may call ${MESSAGE_WRAPPER} (${file})`);
      }
      const method = memberMethod(node.callee);
      if (method && VISIBLE_METHODS.has(method) && isOutsideVisibleCall(node.callee, method)) {
        throw new Error(`visible host sink ${method} is outside ${ADAPTER_FILE} (${file})`);
      }
      if (isDynamicKnownReceiver(node.callee)) {
        throw new Error(`dynamic UI/window access is outside ${ADAPTER_FILE} (${file})`);
      }
      if (isCallMethod(node, "call") || isCallMethod(node, "apply") || isCallMethod(node, "bind")) {
        if (isVisibleMember(node.callee.object)) {
          throw new Error(`visible host sink aliases are outside ${ADAPTER_FILE} (${file})`);
        }
      }
    }
    if (node.type === "VariableDeclarator" || node.type === "AssignmentExpression") {
      const right = node.type === "VariableDeclarator" ? node.init : node.right;
      const left = node.type === "VariableDeclarator" ? node.id : node.left;
      if (isMessageIdentifier(left) || containsVisiblePattern(left) || isMessageIdentifier(right) ||
          isMessageMember(right) || isVisibleMember(right) || isDynamicKnownReceiver(right) ||
          isHostBindingName(left)) {
        throw new Error(`visible host sink acquisition is outside ${ADAPTER_FILE} (${file})`);
      }
    }
    if (node.type === "CallExpression" && node.callee.type === "MemberExpression" &&
        node.callee.computed && (isWindowReceiver(node.callee.object) ||
          isUiReceiver(node.callee.object) || !isClearlyUnrelatedAppend(node.callee.object))) {
      throw new Error(`computed visible host sink access is outside ${ADAPTER_FILE} (${file})`);
    }
    if (node.type === "MemberExpression" && node.computed && isDynamicKnownReceiver(node)) {
      throw new Error(`dynamic UI/window access is outside ${ADAPTER_FILE} (${file})`);
    }
    if (node.type === "Property" && (node.key.type === "Identifier" || node.key.type === "Literal") &&
        (VISIBLE_METHODS.has(String(node.key.name ?? node.key.value)) ||
          String(node.key.name ?? node.key.value) === MESSAGE_WRAPPER) &&
        (node.value.type === "Identifier" || node.shorthand)) {
      throw new Error(`visible host sink destructuring is outside ${ADAPTER_FILE} (${file})`);
    }
  });
}

function isOutsideVisibleCall(callee, method) {
  if (MESSAGE_METHODS.has(method) || method === "createOutputChannel") return true;
  if (method === "appendLine") return true;
  return method === "append" && !isClearlyUnrelatedAppend(callee.object);
}

function isClearlyUnrelatedAppend(receiver) {
  const root = receiverRoot(receiver);
  return root?.type === "Identifier" && ["builder", "base"].includes(root.name);
}

function isVisibleMember(node) {
  if (node?.type === "ChainExpression") return isVisibleMember(node.expression);
  if (node?.type === "CallExpression") {
    return isCallMethod(node, "call") || isCallMethod(node, "apply") || isCallMethod(node, "bind")
      ? isVisibleMember(node.callee.object) : false;
  }
  if (node?.type !== "MemberExpression") return false;
  const method = memberMethod(node) ?? staticMemberMethod(node);
  return VISIBLE_METHODS.has(method) || isDynamicKnownReceiver(node) || isComputedOutputAccess(node) ||
    (isVisibleMember(node.object) && memberMethod(node) === "bind");
}

function isComputedOutputAccess(node) {
  if (node?.type === "ChainExpression") return isComputedOutputAccess(node.expression);
  if (node?.type !== "MemberExpression" || !node.computed) return false;
  const root = receiverRoot(node.object);
  return root?.type === "Identifier" &&
    ["output", "channel", "outputChannel"].includes(root.name);
}

function containsVisiblePattern(pattern) {
  if (!pattern) return false;
  if (pattern.type === "ObjectPattern") return pattern.properties.some((property) =>
    property.type === "RestElement" ||
    VISIBLE_METHODS.has(String(property.key?.name ?? property.key?.value)) ||
    String(property.key?.name ?? property.key?.value) === MESSAGE_WRAPPER ||
    containsVisiblePattern(property.value));
  if (pattern.type === "ArrayPattern") return pattern.elements.some(containsVisiblePattern);
  return false;
}

function isMessageIdentifier(node) {
  return node?.type === "Identifier" && node.name === MESSAGE_WRAPPER;
}

function isMessageMember(node) {
  if (node?.type === "ChainExpression") return isMessageMember(node.expression);
  return node?.type === "MemberExpression" &&
    (memberMethod(node) ?? staticMemberMethod(node)) === MESSAGE_WRAPPER;
}

function isHostBindingName(node) {
  return node?.type === "Identifier" && ["output", "channel", "outputChannel"].includes(node.name);
}

function isMessageCallName(callee) {
  if (callee?.type === "ChainExpression") return isMessageCallName(callee.expression);
  return callee?.type === "Identifier" && callee.name === MESSAGE_WRAPPER ||
    isMessageMember(callee);
}

function isDynamicKnownReceiver(node) {
  if (node?.type === "ChainExpression") return isDynamicKnownReceiver(node.expression);
  return node?.type === "MemberExpression" && node.computed &&
    (isWindowReceiver(node.object) || isUiReceiver(node.object));
}

function isWindowReceiver(node) {
  return node?.type === "MemberExpression" && !node.computed && node.property.type === "Identifier" &&
    node.property.name === "window";
}

function isUiReceiver(node) {
  if (node?.type === "Identifier") return ["ui", "userInterface"].includes(node.name);
  return node?.type === "MemberExpression" && !node.computed &&
    node.property.type === "Identifier" && node.property.name === "userInterface";
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}
