import * as path from "node:path";
import { readFileSync } from "node:fs";
import { integerInRange, protocol } from "./command-protocol.js";
import diagnosticMessages from "./diagnostics.generated.js";
import diagnosticContracts from "./diagnostic-contract.generated.js";

export function applyDiagnostics(api, collection, records, projectRoot, knownUris) {
  applyProjectedDiagnostics(collection, projectDiagnostics(api, records, projectRoot), knownUris);
}

/**
 * Project every record before replacing a collection. A source read/range
 * failure therefore leaves the previous snapshot intact instead of exposing
 * a partially projected or empty result.
 */
export function replaceDiagnostics(api, collection, records, projectRoot, knownUris) {
  const values = projectDiagnostics(api, records, projectRoot);
  const entries = new Map([...knownUris.values()].map((uri) => [uri.toString(), [uri, []]]));
  for (const entry of values.values()) entries.set(entry.uri.toString(), [entry.uri, entry.values]);
  collection.set([...entries.values()]);
  knownUris.clear();
  for (const entry of values.values()) knownUris.set(entry.uri.toString(), entry.uri);
}

function projectDiagnostics(api, records, projectRoot) {
  if (!Array.isArray(records)) throw protocol("invalid_diagnostics");
  const values = new Map();
  for (const record of records) {
    const diagnostic = toDiagnostic(api, record, projectRoot);
    if (!diagnostic) continue;
    const key = diagnostic.uri.toString();
    const entry = values.get(key) ?? { uri: diagnostic.uri, values: [] };
    entry.values.push(diagnostic.value);
    values.set(key, entry);
  }
  return values;
}

function applyProjectedDiagnostics(collection, values, knownUris) {
  collection.set([...values.values()].map((entry) => [entry.uri, entry.values]));
  for (const entry of values.values()) knownUris.set(entry.uri.toString(), entry.uri);
}

export function clearDiagnostics(collection, knownUris) {
  collection.clear?.();
  knownUris.clear();
}

function toDiagnostic(api, record, projectRoot) {
  if (!validDiagnosticRecord(record)) {
    throw protocol("invalid_diagnostic");
  }
  const fullPath = diagnosticPath(projectRoot, record.span.file);
  const open = api.workspace?.textDocuments?.find((document) =>
    document.uri?.scheme === "file" && path.normalize(document.uri.fsPath) === fullPath);
  if (open?.isDirty) return undefined;
  const text = sourceText(api, fullPath);
  const range = rangeForSpan(api, text, record.span);
  const uri = api.Uri.file(fullPath);
  const severityName = record.severity;
  const severity = api.DiagnosticSeverity?.[
    ({ error: "Error", warning: "Warning", information: "Information", hint: "Hint" })[severityName] ?? "Error"
  ] ?? api.DiagnosticSeverity?.Error;
  const value = new api.Diagnostic(
    range,
    diagnosticMessage(record.presentation, record.compatibility_message ?? record.code),
    severity
  );
  value.code = record.code;
  value.source = "recite";
  return { uri, value };
}

function sourceText(api, fullPath) {
  // Structured CLI spans describe the saved on-disk source snapshot. A clean
  // open document is equivalent to that snapshot and avoids a disk race;
  // dirty overlays are deliberately ignored so CLI diagnostics never attach
  // to unsaved text.
  const open = api.workspace?.textDocuments?.find((document) =>
    document.uri?.scheme === "file" && path.normalize(document.uri.fsPath) === fullPath);
  if (open && !open.isDirty && typeof open.getText === "function") return open.getText();
  try {
    return readFileSync(fullPath, "utf8");
  } catch (error) {
    throw protocol("diagnostic_source_unavailable", error.message);
  }
}

function rangeForSpan(api, text, span) {
  const start = positionForScalar(api, text, span.start, "invalid_diagnostic_start");
  if (!span.end) return new api.Range(start, start);
  const endPosition = advanceInclusive(text, span.end);
  const end = positionForScalar(api, text, endPosition, "invalid_diagnostic_end");
  if (end.line < start.line || end.line === start.line && end.character < start.character) {
    throw protocol("invalid_diagnostic_range");
  }
  return new api.Range(start, end);
}

function positionForScalar(api, text, position, errorCode) {
  const lines = text.split("\n");
  const lineIndex = position.line - 1;
  const line = lines[lineIndex]?.endsWith("\r") ? lines[lineIndex].slice(0, -1) : lines[lineIndex];
  if (line === undefined) throw protocol(errorCode);
  const scalarOffset = position.column - 1;
  const values = [...line];
  if (scalarOffset < 0 || scalarOffset > values.length) throw protocol(errorCode);
  const character = values.slice(0, scalarOffset).reduce((total, value) => total + value.length, 0);
  return new api.Position(lineIndex, character);
}

function advanceInclusive(text, position) {
  const lines = text.split("\n");
  const raw = lines[position.line - 1];
  const line = raw?.endsWith("\r") ? raw.slice(0, -1) : raw;
  if (line === undefined) throw protocol("invalid_diagnostic_end");
  const scalarOffset = position.column - 1;
  const values = [...line];
  if (scalarOffset < 0 || scalarOffset >= values.length) {
    throw protocol("invalid_diagnostic_end");
  }
  return { line: position.line, column: position.column + 1 };
}

export function validDiagnosticRecord(record) {
  return Boolean(record && keys(record, ["version", "code", "severity", "span", "presentation", "related", "help", "explanation", "compatibility_message"]) &&
    record.version === 1 && typeof record.code === "string" && /^[A-Z][A-Z0-9]*_[A-Z0-9]+$/u.test(record.code) &&
    ["error", "warning", "information", "hint"].includes(record.severity) && validSpan(record.span) &&
    validPresentation(record.presentation) && Array.isArray(record.related) &&
    record.related.every(validRelatedPresentation) &&
    (record.help === null || record.help === undefined || validPresentation(record.help)) &&
    (record.explanation === null || record.explanation === undefined || validExplanation(record.explanation)) &&
    (record.compatibility_message === null || record.compatibility_message === undefined ||
      typeof record.compatibility_message === "string"));
}

function validSpan(span) {
  return Boolean(span && keys(span, ["file", "start", "end"]) &&
    typeof span.file === "string" && span.file.length > 0 &&
    positive(span.start?.line) && positive(span.start?.column) &&
    (span.end === null || positive(span.end?.line) && positive(span.end?.column) &&
      (span.end.line > span.start.line || span.end.line === span.start.line &&
        span.end.column >= span.start.column)));
}

function validPresentation(presentation) {
  if (!presentation || !keys(presentation, ["id", "arguments"]) ||
      typeof presentation.id !== "string" ||
      !/^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$/u.test(presentation.id) ||
      !presentation.arguments || typeof presentation.arguments !== "object" ||
      Array.isArray(presentation.arguments) ||
      !Object.keys(presentation.arguments).every((name) => /^[a-z][a-z0-9_]*$/u.test(name)) ||
      !Object.values(presentation.arguments).every(validArgument)) return false;
  const definition = diagnosticContracts[presentation.id];
  if (!definition) return true;
  const expected = new Map(definition.arguments.map(({ name, type }) => [name, type]));
  return Object.keys(presentation.arguments).length === expected.size &&
    Object.entries(presentation.arguments).every(([name, argument]) =>
      expected.has(name) && validArgument(argument, expected.get(name)));
}

function validRelatedPresentation(related) {
  return Boolean(related && keys(related, ["span", "presentation"]) &&
    validSpan(related.span) && validPresentation(related.presentation));
}

function validExplanation(explanation) {
  return Boolean(explanation && keys(explanation, ["meaning", "common_causes", "remediation"]) &&
    validPresentation(explanation.meaning) &&
    Array.isArray(explanation.common_causes) && explanation.common_causes.every(validPresentation) &&
    Array.isArray(explanation.remediation) && explanation.remediation.every(validPresentation));
}

function validArgument(argument, expectedType) {
  return Boolean(argument && typeof argument === "object" && keys(argument, ["type", "value"]) &&
    ["string", "integer", "float", "boolean"].includes(argument.type) &&
    (!expectedType || argument.type === expectedType) &&
    (argument.type === "string" ? typeof argument.value === "string" :
      argument.type === "integer" ? integerInRange(argument.value, "-9223372036854775808", "9223372036854775807") :
        argument.type === "float" ? typeof argument.value === "number" && Number.isFinite(argument.value) :
          typeof argument.value === "boolean"));
}

function diagnosticMessage(presentation, fallback) {
  const definition = diagnosticMessages[presentation.id];
  if (!definition) return fallback;
  const template = definition.template;
  const expected = new Map(definition.arguments.map(({ name, type }) => [name, type]));
  if (Object.keys(presentation.arguments).length !== expected.size ||
      Object.entries(presentation.arguments).some(([name, argument]) =>
        !expected.has(name) || !validArgument(argument, expected.get(name)))) return fallback;
  let unresolved = false;
  const rendered = template.replace(/\{\$([a-zA-Z][a-zA-Z0-9_-]*)\}/gu, (_, name) => {
    const argument = presentation.arguments[name];
    if (!argument) unresolved = true;
    return argument ? String(argument.value) : `{${name}}`;
  });
  return unresolved ? fallback : rendered;
}

function diagnosticPath(root, file) {
  const fullPath = path.isAbsolute(file) ? path.normalize(file) : path.resolve(root, file);
  const relative = path.relative(root, fullPath);
  if (!relative || relative === ".." || relative.startsWith(`..${path.sep}`) || path.isAbsolute(relative)) {
    throw protocol("diagnostic_path_outside_project");
  }
  return fullPath;
}

function positive(value) {
  return Number.isSafeInteger(value) && value > 0;
}

function keys(value, expected) {
  return JSON.stringify(Object.keys(value).sort()) === JSON.stringify(expected.slice().sort());
}
