import type { HostApi } from "./types.ts";
import { errorMessage } from "./errors.ts";
import type { DiagnosticCollection, Diagnostic, Uri } from "vscode";
import type {
  DiagnosticRecord,
  DiagnosticSpan,
  Presentation,
  ScalarPosition,
} from "./diagnostic-types.ts";
import { readFileSync } from "node:fs";
import * as path from "node:path";
import { protocol } from "./command-protocol.ts";
import { is, parseWire, diagnosticRecordSchema } from "./schemas/index.js";
import diagnosticMessages from "./diagnostics.generated.js";
const messageDefinitions = new Map(Object.entries(diagnosticMessages));

export function applyDiagnostics(
  api: HostApi,
  collection: Pick<DiagnosticCollection, "set" | "clear">,
  records: unknown,
  projectRoot: string,
  knownUris: Map<string, Uri>,
) {
  applyProjectedDiagnostics(collection, projectDiagnostics(api, records, projectRoot), knownUris);
}

/**
 * Project every record before replacing a collection. A source read/range
 * failure therefore leaves the previous snapshot intact instead of exposing
 * a partially projected or empty result.
 */
export function replaceDiagnostics(
  api: HostApi,
  collection: Pick<DiagnosticCollection, "set" | "clear">,
  records: unknown,
  projectRoot: string,
  knownUris: Map<string, Uri>,
) {
  const values = projectDiagnostics(api, records, projectRoot);
  const entries = new Map<string, [Uri, Diagnostic[]]>(
    [...knownUris.values()].map((uri) => [uri.toString(), [uri, []]]),
  );
  for (const entry of values.values()) entries.set(entry.uri.toString(), [entry.uri, entry.values]);
  collection.set([...entries.values()]);
  knownUris.clear();
  for (const entry of values.values()) knownUris.set(entry.uri.toString(), entry.uri);
}

function projectDiagnostics(api: HostApi, records: unknown, projectRoot: string) {
  if (!Array.isArray(records)) throw protocol("invalid_diagnostics");
  const values = new Map<string, { uri: Uri; values: Diagnostic[] }>();
  const sources = new Map<string, string[]>();
  for (const record of records) {
    const diagnostic = toDiagnostic(api, record, projectRoot, sources);
    if (!diagnostic) continue;
    const key = diagnostic.uri.toString();
    const entry = values.get(key) ?? { uri: diagnostic.uri, values: [] as Diagnostic[] };
    entry.values.push(diagnostic.value);
    values.set(key, entry);
  }
  return values;
}

function applyProjectedDiagnostics(
  collection: Pick<DiagnosticCollection, "set" | "clear">,
  values: Map<string, { uri: Uri; values: Diagnostic[] }>,
  knownUris: Map<string, Uri>,
) {
  collection.set([...values.values()].map((entry) => [entry.uri, entry.values]));
  for (const entry of values.values()) knownUris.set(entry.uri.toString(), entry.uri);
}

export function clearDiagnostics(
  collection: Pick<DiagnosticCollection, "set" | "clear">,
  knownUris: Map<string, Uri>,
) {
  collection.clear?.();
  knownUris.clear();
}

function toDiagnostic(
  api: HostApi,
  record: unknown,
  projectRoot: string,
  sources: Map<string, string[]>,
) {
  const parsed = parseWire(diagnosticRecordSchema, record, "invalid_diagnostic");
  const fullPath = diagnosticPath(projectRoot, parsed.span.file);
  const open = api.workspace?.textDocuments?.find(
    (document) =>
      document.uri?.scheme === "file" && path.normalize(document.uri.fsPath) === fullPath,
  );
  if (open?.isDirty) return undefined;
  let lines = sources.get(fullPath);
  if (!lines) {
    lines = sourceText(api, fullPath).split(/\r\n|\r|\n/u);
    sources.set(fullPath, lines);
  }
  const range = rangeForSpan(api, lines, parsed.span);
  const uri = api.Uri.file(fullPath);
  const severityName = parsed.severity;
  const severity =
    api.DiagnosticSeverity?.[
      ({ error: "Error", warning: "Warning", information: "Information", hint: "Hint" } as const)[
        severityName
      ] ?? "Error"
    ] ?? api.DiagnosticSeverity?.Error;
  const value = new api.Diagnostic(
    range,
    diagnosticMessage(parsed.presentation, parsed.compatibility_message ?? parsed.code),
    severity,
  );
  value.code = parsed.code;
  value.source = "recite";
  return { uri, value };
}

function sourceText(api: HostApi, fullPath: string) {
  // Structured CLI spans describe the saved on-disk source snapshot. A clean
  // open document is equivalent to that snapshot and avoids a disk race;
  // dirty overlays are deliberately ignored so CLI diagnostics never attach
  // to unsaved text.
  const open = api.workspace?.textDocuments?.find(
    (document) =>
      document.uri?.scheme === "file" && path.normalize(document.uri.fsPath) === fullPath,
  );
  if (open && !open.isDirty && typeof open.getText === "function") return open.getText();
  try {
    return readFileSync(fullPath, "utf8");
  } catch (error) {
    throw protocol("diagnostic_source_unavailable", errorMessage(error));
  }
}

function rangeForSpan(api: HostApi, lines: string[], span: DiagnosticSpan) {
  const start = positionForScalar(api, lines, span.start, "invalid_diagnostic_start", false);
  if (!span.end) return new api.Range(start, start);
  const end = positionForScalar(api, lines, span.end, "invalid_diagnostic_end", true);
  if (end.line < start.line || (end.line === start.line && end.character < start.character)) {
    throw protocol("invalid_diagnostic_range");
  }
  return new api.Range(start, end);
}

function positionForScalar(
  api: HostApi,
  lines: string[],
  position: ScalarPosition,
  errorCode: string,
  inclusive: boolean,
) {
  const lineIndex = position.line - 1;
  const line = lines[lineIndex];
  if (line === undefined) throw protocol(errorCode);
  const target = position.column - 1;
  let scalarOffset = 0;
  let character = 0;
  for (const value of line) {
    if (scalarOffset === target) {
      return new api.Position(lineIndex, character + (inclusive ? value.length : 0));
    }
    scalarOffset += 1;
    character += value.length;
  }
  if (!inclusive && scalarOffset === target) return new api.Position(lineIndex, character);
  throw protocol(errorCode);
}

export function validDiagnosticRecord(value: unknown): value is DiagnosticRecord {
  return is(diagnosticRecordSchema, value);
}

function diagnosticMessage(presentation: Presentation, fallback: string) {
  const definition = messageDefinitions.get(presentation.id);
  if (!definition) return fallback;
  const template = definition.template;
  const expected = new Map(definition.arguments.map(({ name, type }) => [name, type]));
  if (
    Object.keys(presentation.arguments).length !== expected.size ||
    Object.entries(presentation.arguments).some(
      ([name, argument]) => !expected.has(name) || argument.type !== expected.get(name),
    )
  )
    return fallback;
  let unresolved = false;
  const rendered = template.replace(/\{\$([a-zA-Z][a-zA-Z0-9_-]*)\}/gu, (_, name) => {
    const argument = presentation.arguments[name];
    if (!argument) unresolved = true;
    return argument ? String(argument.value) : `{${name}}`;
  });
  return unresolved ? fallback : rendered;
}

function diagnosticPath(root: string, file: string) {
  const fullPath = path.isAbsolute(file) ? path.normalize(file) : path.resolve(root, file);
  const relative = path.relative(root, fullPath);
  if (
    !relative ||
    relative === ".." ||
    relative.startsWith(`..${path.sep}`) ||
    path.isAbsolute(relative)
  ) {
    throw protocol("diagnostic_path_outside_project");
  }
  return fullPath;
}
