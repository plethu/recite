import { parse } from "@fluent/syntax";

/** Project only inline text and simple variables from valid canonical Fluent. */
export function parseRepresentableMessages(source, ids, host = "editor") {
  const canonical = canonicalMessages(source);
  const messages = new Map();
  for (const id of new Set(ids)) {
    const message = canonical.get(id);
    if (!message) throw new Error(`canonical Fluent message is missing ${id}`);
    messages.set(id, projectTemplate(source, message, host));
  }
  return messages;
}

/** Unsupported valid expressions retain the diagnostic host's fallback. */
export function parseDiagnosticMessages(source, host = "editor") {
  const messages = new Map();
  for (const [id, message] of canonicalMessages(source)) {
    if (!/^diagnostic-[a-z0-9-]+$/u.test(id)) continue;
    if (/(?:-help|-related|-meaning|-cause-\d+|-remediation-\d+)$/u.test(id)) continue;
    try {
      messages.set(id, projectTemplate(source, message, host));
    } catch (error) {
      if (!(error instanceof UnsupportedExpression)) throw error;
    }
  }
  return messages;
}

export function parseDiagnosticContracts(source) {
  const contracts = new Map();
  for (const line of source.split(/\r?\n/u)) {
    if (!line || line.startsWith("#")) continue;
    const [id, name = "", type = ""] = line.split("\t");
    const argumentsForId = contracts.get(id) ?? [];
    if (name) argumentsForId.push({ name, type });
    contracts.set(id, argumentsForId);
  }
  return contracts;
}

export function assertDiagnosticTemplate(id, template, argumentsForId) {
  const placeholders = [...template.matchAll(/\{\s*\$([a-zA-Z][a-zA-Z0-9_-]*)\s*\}/gu)]
    .map((match) => match[1])
    .filter((name, index, values) => values.indexOf(name) === index)
    .sort();
  const contractNames = argumentsForId.map(({ name }) => name).sort();
  if (JSON.stringify(placeholders) !== JSON.stringify(contractNames)) {
    throw new Error(`diagnostic contract/template mismatch for ${id}`);
  }
}

function canonicalMessages(source) {
  const messages = new Map();
  for (const entry of parse(source).body) {
    if (entry.type === "Junk") {
      const annotation = entry.annotations[0];
      throw new Error(
        `malformed canonical Fluent at line ${
          lineAt(source, annotation?.span.start ?? entry.span.start)
        }: ${annotation?.message ?? "invalid syntax"}`,
      );
    }
    if (entry.type !== "Message") continue;
    if (messages.has(entry.id.name)) {
      throw new Error(
        `duplicate canonical Fluent message ${entry.id.name} at line ${
          lineAt(source, entry.id.span.start)
        }`,
      );
    }
    messages.set(entry.id.name, entry);
  }
  return messages;
}

function projectTemplate(source, message, host) {
  if (message.attributes.length > 0) {
    throw new Error(messageProblem(source, message, "uses an unsupported attribute"));
  }
  const elements = message.value.elements;
  if (
    !elements.every((element) =>
      element.type === "TextElement"
      || (element.type === "Placeable" && element.expression.type === "VariableReference")
    )
  ) {
    throw new UnsupportedExpression(
      messageProblem(source, message, "uses an unsupported expression"),
    );
  }
  if (/[\r\n]/u.test(source.slice(message.id.span.start, message.value.span.end))) {
    throw new Error(
      `${
        messageProblem(source, message, "uses a continuation")
      }; ${host} projections support only single-line templates`,
    );
  }
  return elements.map((element) =>
    element.type === "TextElement" ? element.value : `{$${element.expression.id.name}}`
  ).join("");
}

function messageProblem(source, message, problem) {
  return `canonical Fluent message ${message.id.name} ${problem} at line ${
    lineAt(source, message.id.span.start)
  }`;
}

function lineAt(source, offset) {
  return source.slice(0, offset).split(/\r?\n/u).length;
}

class UnsupportedExpression extends Error {}
