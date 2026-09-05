import test from "node:test";
import assert from "node:assert/strict";
import { mkdir, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { applyDiagnostics, replaceDiagnostics, validDiagnosticRecord } from "../src/command-diagnostics.js";

test("diagnostic spans are inclusive and project to VS Code UTF-16 ranges", async () => {
  const root = await tempRoot("ascii");
  const file = path.join(root, "dialogue.recite");
  await writeFile(file, "abc\r\n", "utf8");
  const entries = new Map();
  const api = fakeApi(entries, []);
  applyDiagnostics(api, collection(entries), [diagnostic(file, {
    line: 1, column: 2
  }, { line: 1, column: 2 }, "diagnostic-parse-001")], root, new Map());
  const value = entries.get(`file://${file}`)[0];
  assert.equal(value.range.start.line, 0);
  assert.equal(value.range.start.character, 1);
  assert.equal(value.range.end.line, 0);
  assert.equal(value.range.end.character, 2);
  await rm(root, { recursive: true, force: true });
});

test("non-BMP scalars and CRLF use source text for stable UTF-16 columns", async () => {
  const root = await tempRoot("unicode");
  const file = path.join(root, "dialogue.recite");
  const source = "😀ab\r\nnext";
  await writeFile(file, source, "utf8");
  const entries = new Map();
  const api = fakeApi(entries, []);
  applyDiagnostics(api, collection(entries), [diagnostic(file, {
    line: 1, column: 2
  }, { line: 1, column: 3 }, "diagnostic-validate-007", {
    reference: { type: "string", value: "missing" }
  }, "wrong compatibility")], root, new Map());
  const value = entries.get(`file://${file}`)[0];
  assert.equal(value.range.start.line, 0);
  assert.equal(value.range.start.character, 2);
  assert.equal(value.range.end.line, 0);
  assert.equal(value.range.end.character, 4);
  assert.equal(value.message, "unknown block reference `missing`");
  await rm(root, { recursive: true, force: true });
});

test("replacement projects before clearing the previous snapshot", async () => {
  const root = await tempRoot("atomic");
  const file = path.join(root, "dialogue.recite");
  await writeFile(file, "ok\n", "utf8");
  const entries = new Map();
  const knownUris = new Map();
  const api = fakeApi(entries, []);
  const target = collection(entries);
  applyDiagnostics(api, target, [diagnostic(file, { line: 1, column: 1 }, null, "diagnostic-parse-001")], root, knownUris);
  assert.equal(entries.size, 1);
  assert.equal(target.calls.length, 1);
  assert.throws(() => replaceDiagnostics(api, target, [diagnostic("missing.recite", {
    line: 1, column: 1
  }, null, "diagnostic-parse-001")], root, knownUris), /diagnostic_source_unavailable/);
  assert.equal(entries.size, 1);
  assert.equal(target.calls.length, 1, "failed projection must not submit a replacement batch");
  await rm(root, { recursive: true, force: true });
});

test("replacement submits one complete batch and clears removed URIs atomically", async () => {
  const root = await tempRoot("batch");
  const first = path.join(root, "first.recite");
  const second = path.join(root, "second.recite");
  await writeFile(first, "ok\n", "utf8");
  await writeFile(second, "ok\n", "utf8");
  const entries = new Map();
  const knownUris = new Map();
  const target = collection(entries);
  const api = fakeApi(entries, []);
  applyDiagnostics(api, target, [diagnostic(first, { line: 1, column: 1 }, null, "diagnostic-parse-001")], root, knownUris);
  replaceDiagnostics(api, target, [diagnostic(second, { line: 1, column: 1 }, null, "diagnostic-parse-001")], root, knownUris);
  assert.equal(target.calls.length, 2);
  assert.deepEqual(target.calls[1].map(([uri, values]) => [uri.fsPath, values.length]), [
    [first, 0], [second, 1]
  ]);
  assert.equal(entries.has(`file://${first}`), false);
  assert.equal(entries.has(`file://${second}`), true);
  await rm(root, { recursive: true, force: true });
});

test("dirty open overlays do not receive disk-backed command diagnostics", async () => {
  const root = await tempRoot("dirty");
  const file = path.join(root, "dialogue.recite");
  await writeFile(file, "disk source\n", "utf8");
  const entries = new Map();
  const knownUris = new Map();
  const open = {
    isDirty: true,
    uri: { scheme: "file", fsPath: file, toString: () => `file://${file}` },
    getText: () => "😀changed overlay\n"
  };
  const api = fakeApi(entries, [open]);
  const target = collection(entries);
  applyDiagnostics(api, target, [diagnostic(file, { line: 1, column: 1 }, null, "diagnostic-parse-001")], root, knownUris);
  assert.equal(entries.size, 0);
  await rm(root, { recursive: true, force: true });
});

test("typed diagnostic contracts cover auxiliary presentations and reject wrong argument types", () => {
  const valid = diagnostic("dialogue.recite", { line: 1, column: 1 }, null,
    "diagnostic-validate-024-help", { tag: { type: "string", value: "b" } });
  assert.equal(validDiagnosticRecord(valid), true);
  const wrong = structuredClone(valid);
  wrong.presentation.arguments.tag = { type: "integer", value: 7 };
  assert.equal(validDiagnosticRecord(wrong), false);
});

async function tempRoot(name) {
  const root = path.join(os.tmpdir(), `recite-command-diagnostics-${name}-${process.pid}`);
  await mkdir(root, { recursive: true });
  return root;
}

function diagnostic(file, start, end, id, arguments_ = {}, compatibility = "fallback") {
  return {
    version: 1, code: "RECITE_TEST001", severity: "error",
    span: { file, start, end }, presentation: { id, arguments: arguments_ },
    related: [], help: null, explanation: null, compatibility_message: compatibility
  };
}

function fakeApi(entries, textDocuments) {
  return {
    workspace: { textDocuments },
    Uri: { file: (fsPath) => ({ fsPath, toString: () => `file://${fsPath}` }) },
    Position: class Position { constructor(line, character) { this.line = line; this.character = character; } },
    Range: class Range { constructor(start, end) { this.start = start; this.end = end; } },
    Diagnostic: class Diagnostic { constructor(range, message, severity) { Object.assign(this, { range, message, severity }); } },
    DiagnosticSeverity: { Error: "error", Warning: "warning", Information: "info", Hint: "hint" }
  };
}

function collection(entries) {
  const calls = [];
  return {
    calls,
    set: (batch) => {
      assert.equal(Array.isArray(batch), true, "diagnostic replacement must use one batch");
      calls.push(batch);
      entries.clear();
      for (const [uri, values] of batch) {
        if (values.length > 0) entries.set(uri.toString(), values);
      }
    },
    clear: () => { throw new Error("replacement must not clear the collection"); }
  };
}
