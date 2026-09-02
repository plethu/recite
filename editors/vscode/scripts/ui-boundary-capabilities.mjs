import { memberMethod, staticMemberMethod } from "./ui-boundary-ast.mjs";

const REFLECT_METHODS = new Set([
  "apply", "construct", "defineProperty", "deleteProperty", "get",
  "getOwnPropertyDescriptor", "getPrototypeOf", "has", "isExtensible",
  "ownKeys", "preventExtensions", "set", "setPrototypeOf"
]);

export function isReflectCall(node) {
  const callee = unwrap(node.callee);
  return callee?.type === "MemberExpression" &&
    callee.object.type === "Identifier" && callee.object.name === "Reflect" &&
    REFLECT_METHODS.has(staticMethod(callee));
}

export function isDynamicReflectCall(node) {
  const callee = unwrap(node.callee);
  return callee?.type === "MemberExpression" && callee.computed &&
    callee.object.type === "Identifier" && callee.object.name === "Reflect" &&
    !staticMethod(callee);
}

export function isWindowPropertyLookup(node) {
  if (!isReflectCall(node) || node.arguments.length < 2) return false;
  const target = unwrap(node.arguments[0]);
  const property = unwrap(node.arguments[1]);
  return target?.type === "Identifier" && ["api", "vscode"].includes(target.name) &&
    (property?.type !== "Literal" || property.value === "window");
}

function staticMethod(node) {
  const current = unwrap(node);
  return memberMethod(current) ?? staticMemberMethod(current);
}

function unwrap(node) {
  return node?.type === "ChainExpression" ? node.expression : node;
}
