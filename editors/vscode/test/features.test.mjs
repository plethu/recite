import test from "node:test";
import assert from "node:assert/strict";
import {
  lspDiagnosticToVscode,
  lspWorkspaceEditToVscode
} from "../src/lsp-features.js";

test("diagnostics retain stable code, source, severity, and UTF-16 range", () => {
  const diagnostic = lspDiagnosticToVscode(api, {
    range: { start: { line: 2, character: 11 }, end: { line: 2, character: 13 } },
    severity: 2,
    code: "RECITE_PARSE007",
    source: "recite-lsp",
    message: "expected a statement"
  });
  assert.deepEqual({
    range: {
      start: { line: diagnostic.range.start.line, character: diagnostic.range.start.character },
      end: { line: diagnostic.range.end.line, character: diagnostic.range.end.character }
    },
    message: diagnostic.message,
    severity: diagnostic.severity,
    code: diagnostic.code,
    source: diagnostic.source
  }, {
    range: { start: { line: 2, character: 11 }, end: { line: 2, character: 13 } },
    message: "expected a statement",
    severity: "warning",
    code: "RECITE_PARSE007",
    source: "recite-lsp"
  });
});

test("versioned workspace edits are refused for a stale open document", () => {
  const uri = api.Uri.parse("file:///workspace/dialogue.recite");
  const stale = lspWorkspaceEditToVscode(api, {
    documentChanges: [{
      textDocument: { uri: uri.toString(), version: 3 },
      edits: [{
        range: { start: { line: 1, character: 0 }, end: { line: 1, character: 4 } },
        newText: "done"
      }]
    }]
  }, () => ({ version: 4 }));
  assert.equal(stale, undefined);

  const current = lspWorkspaceEditToVscode(api, {
    documentChanges: [{
      textDocument: { uri: uri.toString(), version: 4 },
      edits: [{
        range: { start: { line: 1, character: 0 }, end: { line: 1, character: 4 } },
        newText: "done"
      }]
    }]
  }, () => ({ version: 4 }));
  assert.equal(current.replacements.length, 1);
  assert.equal(current.replacements[0].uri.toString(), uri.toString());
});

const api = {
  Position: class Position {
    constructor(line, character) { this.line = line; this.character = character; }
  },
  Range: class Range {
    constructor(start, end) { this.start = start; this.end = end; }
  },
  Uri: class Uri {
    constructor(value) { this.value = value; }
    static parse(value) { return new this(value); }
    toString() { return this.value; }
  },
  Diagnostic: class Diagnostic {
    constructor(range, message, severity) { this.range = range; this.message = message; this.severity = severity; }
  },
  DiagnosticSeverity: { Error: "error", Warning: "warning", Information: "info", Hint: "hint" },
  WorkspaceEdit: class WorkspaceEdit {
    constructor() { this.replacements = []; }
    replace(uri, range, newText) { this.replacements.push({ uri, range, newText }); }
  }
};
