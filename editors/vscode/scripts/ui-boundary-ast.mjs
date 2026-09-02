import { parse } from "acorn";

export function parseSource(source, file) {
  try {
    return parse(source, { ecmaVersion: "latest", sourceType: "module", allowHashBang: true });
  } catch (error) {
    throw new Error(`invalid JavaScript in ${file}: ${error.message}`, { cause: error });
  }
}

export function walkWithParents(node, visit, parent = undefined, ancestors = []) {
  if (!node || typeof node !== "object") return;
  visit(node, parent, ancestors);
  const next = [...ancestors, node];
  for (const [key, value] of Object.entries(node)) {
    if (["start", "end", "loc"].includes(key)) continue;
    if (Array.isArray(value)) {
      for (const child of value) if (child?.type) walkWithParents(child, visit, node, next);
    } else if (value?.type) walkWithParents(value, visit, node, next);
  }
}

export function memberMethod(node) {
  if (node?.type !== "MemberExpression" || node.computed || node.property.type !== "Identifier") {
    return undefined;
  }
  return node.property.name;
}

export function staticMemberMethod(node) {
  if (node?.type !== "MemberExpression" || !node.computed || node.property.type !== "Literal" ||
      typeof node.property.value !== "string") return undefined;
  return node.property.value;
}

export function staticModuleSpecifier(node) {
  if (![
    "ImportDeclaration",
    "ExportNamedDeclaration",
    "ExportAllDeclaration",
    "ImportExpression"
  ].includes(node?.type)) return undefined;
  const source = node.source;
  if (source?.type === "Literal" && typeof source.value === "string") return source.value;
  if (node.type !== "ImportExpression" || source?.type !== "TemplateLiteral" ||
      source.expressions.length !== 0 || source.quasis.length !== 1) return undefined;
  const value = source.quasis[0]?.value?.cooked;
  return typeof value === "string" ? value : undefined;
}

export function isCallMethod(node, method) {
  return node?.type === "CallExpression" && node.callee.type === "MemberExpression" &&
    !node.callee.computed && node.callee.property.type === "Identifier" &&
    node.callee.property.name === method;
}

export function receiverRoot(node) {
  if (node?.type === "Identifier") return node;
  if (node?.type === "MemberExpression") return receiverRoot(node.object);
  return undefined;
}

export function propertyName(node) {
  if (node?.type !== "Property" || node.computed) return undefined;
  return node.key.type === "Identifier" ? node.key.name :
    node.key.type === "Literal" && typeof node.key.value === "string" ? node.key.value : undefined;
}

export function isMemberCall(node, root, middle, method) {
  return node?.type === "CallExpression" && node.callee.type === "MemberExpression" &&
    !node.callee.computed && !node.callee.optional && node.callee.property.type === "Identifier" &&
    node.callee.property.name === method && node.callee.object.type === "MemberExpression" &&
    !node.callee.object.computed && !node.callee.object.optional &&
    node.callee.object.property.type === "Identifier" &&
    node.callee.object.property.name === middle && node.callee.object.object.type === "Identifier" &&
    node.callee.object.object.name === root;
}
