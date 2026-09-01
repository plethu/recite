import { parse } from "acorn";

const MESSAGE_WRAPPER = "clientMessage";
const VISIBLE_OUTPUT_METHODS = new Set(["append", "appendLine", "createOutputChannel"]);
const VISIBLE_MESSAGE_METHODS = new Set([
  "showWarningMessage",
  "showErrorMessage",
  "showInformationMessage"
]);
const VISIBLE_METHODS = new Set([
  ...VISIBLE_OUTPUT_METHODS,
  ...VISIBLE_MESSAGE_METHODS
]);

export function parseSource(source, file) {
  try {
    return parse(source, {
      ecmaVersion: "latest",
      sourceType: "module",
      allowHashBang: true
    });
  } catch (error) {
    throw new Error(`invalid JavaScript in ${file}: ${error.message}`, { cause: error });
  }
}

export function resolveBindings(ast) {
  const bindings = new Map();
  const sources = new Map();

  walk(ast, (node) => {
    if (node.type === "ImportDeclaration") {
      const moduleName = node.source.value;
      for (const specifier of node.specifiers) {
        const name = specifier.local.name;
        if (specifier.type === "ImportNamespaceSpecifier") {
          bindings.set(name, { kind: moduleName === "./messages.js" ? "namespace" : "unknown" });
        } else if (specifier.type === "ImportDefaultSpecifier" && moduleName === "./messages.js") {
          bindings.set(name, { kind: "namespace" });
        } else if (moduleName === "./messages.js" &&
            specifier.imported?.name === MESSAGE_WRAPPER) {
          bindings.set(name, { kind: "canonical" });
        } else if (moduleName.endsWith("/messages.generated.js") ||
            moduleName === "./messages.generated.js") {
          bindings.set(name, { kind: "generated" });
        } else {
          bindings.set(name, { kind: "unknown" });
        }
      }
    }
    if (node.type === "VariableDeclarator") {
      declarePattern(node.id, node.init, bindings, sources);
    }
    if (node.type === "FunctionDeclaration" || node.type === "ClassDeclaration") {
      if (node.id) declareUnknown(node.id.name, bindings);
    }
    if (node.type === "FunctionDeclaration" || node.type === "FunctionExpression" ||
        node.type === "ArrowFunctionExpression") {
      for (const parameter of node.params) {
        declareParameterPattern(parameter, bindings, sources);
      }
    }
    if (node.type === "CatchClause" && node.param) {
      declareParameterPattern(node.param, bindings, sources);
    }
    if (node.type === "AssignmentExpression") {
      assignPattern(node.left, node.right, bindings, sources);
    }
  });

  // Resolve a bounded alias graph. Unresolved values stay unknown and are
  // handled fail-closed by the policy facade when used at a visible boundary.
  for (let pass = 0; pass < bindings.size + 1; pass += 1) {
    let changed = false;
    for (const [name, expression] of sources) {
      const next = expressionKind(expression, bindings);
      const current = bindings.get(name);
      if (!current || current.kind === next) continue;
      if (next === "unknown" && current.kind !== "unknown") continue;
      bindings.set(name, {
        kind: next,
        method: methodFromExpression(expression, bindings),
        value: next === "text" ? staticStringValue(expression, bindings) : undefined
      });
      changed = true;
    }
    if (!changed) break;
  }
  return bindings;
}

export function visibleMethod(callee, bindings) {
  if (callee.type === "ChainExpression") return visibleMethod(callee.expression, bindings);
  if (callee.type === "MemberExpression" || callee.type === "OptionalMemberExpression") {
    const property = staticPropertyName(callee, bindings);
    if (property && VISIBLE_METHODS.has(property)) return property;
    if (callee.computed && property === undefined) return "ambiguous-visible";
    if (property === "call" || property === "apply" || property === "bind") {
      const method = visibleMethod(callee.object, bindings);
      if (method) return method;
    }
    return undefined;
  }
  if (callee.type === "Identifier") {
    const kind = bindingKind(callee.name, bindings);
    if (kind === "visible") return bindings.get(callee.name).method;
    if (kind === "ambiguous-visible") return "ambiguous-visible";
    if (VISIBLE_METHODS.has(callee.name)) return callee.name;
  }
  return undefined;
}

export function isSourceOwnedMessage(expression, bindings) {
  if (!expression) return false;
  if (expression.type === "ChainExpression") {
    return isSourceOwnedMessage(expression.expression, bindings);
  }
  if (expression.type === "CallExpression") {
    return expressionKind(expression.callee, bindings) === "canonical";
  }
  if (expression.type === "TemplateLiteral") {
    return expression.quasis.every((quasi) => quasi.value.raw === "") &&
      expression.expressions.length === 1 &&
      isSourceOwnedMessage(expression.expressions[0], bindings);
  }
  return false;
}

export function isStaticText(expression, bindings) {
  if (!expression) return false;
  if (expression.type === "Identifier") {
    return ["text", "static-text"].includes(bindingKind(expression.name, bindings));
  }
  if (expression.type === "Literal") return typeof expression.value === "string";
  if (expression.type === "TemplateLiteral") {
    return expression.expressions.length === 0 ||
      expression.quasis.some((quasi) => quasi.value.raw !== "");
  }
  if (expression.type === "BinaryExpression" && expression.operator === "+") {
    return isStaticText(expression.left, bindings) || isStaticText(expression.right, bindings);
  }
  return false;
}

export function staticPropertyName(node, bindings) {
  if (!node.computed && node.property?.type === "Identifier") return node.property.name;
  return node.computed ? staticStringValue(node.property, bindings) : undefined;
}

export function staticStringValue(expression, bindings) {
  if (!expression) return undefined;
  if (expression.type === "Identifier") {
    return bindings.get(expression.name)?.kind === "text"
      ? bindings.get(expression.name).value
      : undefined;
  }
  if (expression.type === "Literal" && typeof expression.value === "string") return expression.value;
  if (expression.type === "TemplateLiteral") {
    let value = "";
    for (let index = 0; index < expression.quasis.length; index += 1) {
      const quasi = expression.quasis[index].value.cooked;
      if (quasi === null) return undefined;
      value += quasi;
      if (index < expression.expressions.length) {
        const part = staticStringValue(expression.expressions[index], bindings);
        if (part === undefined) return undefined;
        value += part;
      }
    }
    return value;
  }
  if (expression.type === "BinaryExpression" && expression.operator === "+") {
    const left = staticStringValue(expression.left, bindings);
    const right = staticStringValue(expression.right, bindings);
    return left === undefined || right === undefined ? undefined : left + right;
  }
  return undefined;
}

function declarePattern(pattern, init, bindings, sources) {
  if (pattern.type === "Identifier") {
    declareUnknown(pattern.name, bindings);
    if (init) sources.set(pattern.name, init);
    return;
  }
  if (pattern.type === "AssignmentPattern") {
    declarePattern(pattern.left, init ?? pattern.right, bindings, sources);
    return;
  }
  if (pattern.type === "ObjectPattern") {
    for (const property of pattern.properties) {
      if (property.type === "RestElement") {
        declarePattern(property.argument, undefined, bindings, sources);
        continue;
      }
      const name = staticKeyName(property, bindings);
      declareDestructuredPattern(property.value, name, init, bindings, sources);
    }
    return;
  }
  if (pattern.type === "ArrayPattern") {
    for (const element of pattern.elements) {
      if (element) declarePattern(element, undefined, bindings, sources);
    }
  }
}

function declareDestructuredPattern(pattern, name, init, bindings, sources) {
  if (pattern.type === "AssignmentPattern") {
    declareDestructuredPattern(pattern.left, name, init, bindings, sources);
    return;
  }
  if (pattern.type === "Identifier") {
    declareUnknown(pattern.name, bindings);
    const kind = name && VISIBLE_METHODS.has(name)
      ? { kind: "visible", method: name }
      : name === MESSAGE_WRAPPER && expressionKind(init, bindings) === "namespace"
        ? { kind: "canonical" }
        : { kind: name ? "unknown" : "ambiguous-visible" };
    bindings.set(pattern.name, kind);
    return;
  }
  declarePattern(pattern, undefined, bindings, sources);
}

function declareParameterPattern(pattern, bindings, sources) {
  if (pattern.type === "Identifier") {
    // Parameters shadow imports and outer bindings. The checker is
    // intentionally conservative across its file-level alias summary.
    bindings.set(pattern.name, { kind: "unknown" });
    return;
  }
  if (pattern.type === "AssignmentPattern") {
    declareParameterPattern(pattern.left, bindings, sources);
    return;
  }
  if (pattern.type === "ObjectPattern") {
    for (const property of pattern.properties) {
      if (property.type === "RestElement") {
        declareParameterPattern(property.argument, bindings, sources);
      } else {
        declareDestructuredPattern(
          property.value, staticKeyName(property, bindings), undefined, bindings, sources
        );
      }
    }
    return;
  }
  if (pattern.type === "ArrayPattern") {
    for (const element of pattern.elements) {
      if (element) declareParameterPattern(element, bindings, sources);
    }
  }
}

function assignPattern(pattern, expression, bindings, sources) {
  if (pattern.type === "Identifier") {
    declareUnknown(pattern.name, bindings);
    sources.set(pattern.name, expression);
    return;
  }
  if (pattern.type === "AssignmentPattern") {
    assignPattern(pattern.left, expression, bindings, sources);
    return;
  }
  if (pattern.type === "ObjectPattern") {
    for (const property of pattern.properties) {
      if (property.type === "Property") assignPattern(property.value, undefined, bindings, sources);
    }
  }
}

function declareUnknown(name, bindings) {
  if (!bindings.has(name)) bindings.set(name, { kind: "unknown" });
}

export function expressionKind(expression, bindings) {
  if (!expression) return "unknown";
  if (expression.type === "ChainExpression") return expressionKind(expression.expression, bindings);
  if (expression.type === "Identifier") return bindingKind(expression.name, bindings);
  if (staticStringValue(expression, bindings) !== undefined) return "text";
  if (isStaticText(expression, bindings)) return "static-text";
  if (expression.type === "MemberExpression" || expression.type === "OptionalMemberExpression") {
    const property = staticPropertyName(expression, bindings);
    if (property && VISIBLE_METHODS.has(property)) return "visible";
    if (property === MESSAGE_WRAPPER && expressionKind(expression.object, bindings) === "namespace") {
      return "canonical";
    }
    if (property === "bind" || property === "call" || property === "apply") {
      if (expressionKind(expression.object, bindings) === "visible") return "visible";
    }
    if (expression.computed && property === undefined) return "ambiguous-visible";
    return "unknown";
  }
  if (expression.type === "AssignmentExpression") return expressionKind(expression.right, bindings);
  if (expression.type === "SequenceExpression") {
    return expressionKind(expression.expressions.at(-1), bindings);
  }
  return "unknown";
}

function bindingKind(name, bindings) {
  if (bindings.has(name)) return bindings.get(name).kind;
  if (name === MESSAGE_WRAPPER) return "canonical";
  return "unknown";
}

function methodFromExpression(expression, bindings) {
  if (expression?.type === "MemberExpression" || expression?.type === "OptionalMemberExpression") {
    const property = staticPropertyName(expression, bindings);
    if (property && VISIBLE_METHODS.has(property)) return property;
    if (property === "bind" || property === "call" || property === "apply") {
      return methodFromExpression(expression.object, bindings);
    }
    return undefined;
  }
  if (expression?.type === "ChainExpression") return methodFromExpression(expression.expression, bindings);
  if (expression?.type === "Identifier") return bindings.get(expression.name)?.method;
  return undefined;
}

function staticKeyName(node, bindings) {
  if (!node.computed && node.key?.type === "Identifier") return node.key.name;
  return node.computed ? staticStringValue(node.key, bindings) :
    node.key?.type === "Literal" && typeof node.key.value === "string" ? node.key.value : undefined;
}

export function walk(node, visitor) {
  if (!node || typeof node !== "object") return;
  visitor(node);
  for (const [key, value] of Object.entries(node)) {
    if (key === "start" || key === "end" || key === "loc") continue;
    if (Array.isArray(value)) {
      for (const child of value) walk(child, visitor);
    } else if (value && typeof value === "object") {
      walk(value, visitor);
    }
  }
}
