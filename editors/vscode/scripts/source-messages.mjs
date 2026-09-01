import {
  isSourceOwnedMessage,
  isStaticText,
  parseSource,
  resolveBindings,
  expressionKind,
  staticPropertyName,
  staticStringValue,
  visibleMethod,
  walk
} from "./source-messages-ast.mjs";

const MESSAGE_WRAPPER = "clientMessage";

/**
 * Check the extension's visible message boundary against the generated
 * projection. Acorn owns JavaScript syntax here; this check must not infer
 * source ownership from comments, strings, regexes, or delimiter counting.
 *
 * Dynamic values from the language server or the host remain valid output
 * payloads. Source-owned text must come from the canonical clientMessage
 * wrapper, and aliases of either boundary are resolved conservatively.
 */
export function assertSourceMessageOwnership(sourceFiles, ownedIds, projectedMessages) {
  const expected = new Set(ownedIds);
  const used = new Map();

  for (const [file, source] of sourceFiles) {
    const ast = parseSource(source, file);
    const bindings = resolveBindings(ast);
    walk(ast, (node) => {
      if (node.type !== "CallExpression") return;
      inspectMessageCall(node, file, bindings, expected, projectedMessages, used);
      inspectVisibleCall(node, file, bindings);
    });
  }

  assert(used.size === expected.size && [...expected].every((id) => used.has(id)),
    `extension source message use must cover exactly the owned IDs; missing ${missing(expected, used)}`);
  for (const [id, files] of used) {
    assert(Object.hasOwn(projectedMessages, id),
      `extension source references a message without a generated projection: ${id} (${files.join(", ")})`);
  }
}

function inspectMessageCall(node, file, bindings, expected, projectedMessages, used) {
  const kind = expressionKindForCall(node, bindings);
  if (kind === "canonical") {
    const id = node.arguments[1]?.type === "Literal" &&
      typeof node.arguments[1].value === "string"
      ? node.arguments[1].value
      : undefined;
    assert(id, `clientMessage must use a literal canonical message ID (${file})`);
    const sites = used.get(id) ?? [];
    sites.push(file);
    used.set(id, sites);
    return;
  }

  // A local function named clientMessage must not masquerade as the wrapper.
  // Also fail closed for an unresolved alias carrying an owned ID: accepting
  // it would make a computed/destructured bypass possible.
  if ((node.callee.type === "Identifier" && node.callee.name === MESSAGE_WRAPPER) ||
      unresolvedOwnedMessageCall(node, bindings, expected, projectedMessages)) {
    throw new Error(`clientMessage must use the canonical wrapper (${file})`);
  }
}

function expressionKindForCall(node, bindings) {
  return expressionKind(node.callee, bindings);
}

function unresolvedOwnedMessageCall(node, bindings, expected, projectedMessages) {
  if (node.arguments.length < 2) return false;
  const id = staticStringValue(node.arguments[1], bindings);
  if (id === undefined) return false;
  return expected.has(id) || Object.hasOwn(projectedMessages, id);
}

function inspectVisibleCall(node, file, bindings) {
  const method = visibleMethod(node.callee, bindings);
  if (!method) return;
  const argument = visibleArgument(node, method, bindings);
  if (!isSourceOwnedMessage(argument, bindings)) {
    if (isStaticText(argument, bindings) || method === "ambiguous-visible") {
      throw new Error(`visible host output ${method} must use clientMessage (${file})`);
    }
  }
}

function visibleArgument(node, method, bindings) {
  const callee = node.callee;
  const property = callee?.type === "MemberExpression"
    ? staticPropertyName(callee, bindings)
    : undefined;
  if (method !== "ambiguous-visible" &&
      (property === "call" || property === "apply")) return node.arguments[1];
  return node.arguments[0];
}

function missing(expected, used) {
  return [...expected].filter((id) => !used.has(id)).join(", ") || "none";
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}
