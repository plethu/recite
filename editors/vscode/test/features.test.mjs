import test from "node:test";
import assert from "node:assert/strict";
import {
  lspDiagnosticToVscode,
  lspWorkspaceEditToVscode,
  vscodeDiagnosticToLsp,
  workspaceEditIsCurrent
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

test("delayed workspace edits revalidate zero-edit sibling preconditions atomically", () => {
  const primary = api.Uri.parse("file:///workspace/dialogue.recite");
  const sibling = api.Uri.parse("file:///workspace/other.recite");
  const documents = new Map([
    [primary.toString(), { version: 4 }],
    [sibling.toString(), { version: 9 }]
  ]);
  const edit = lspWorkspaceEditToVscode(api, {
    documentChanges: [
      {
        textDocument: { uri: primary.toString(), version: 4 },
        edits: [{
          range: { start: { line: 1, character: 0 }, end: { line: 1, character: 4 } },
          newText: "done"
        }]
      },
      { textDocument: { uri: sibling.toString(), version: 9 }, edits: [] }
    ]
  }, (uri) => documents.get(uri.toString()));

  assert.equal(workspaceEditIsCurrent(edit), true);
  documents.get(sibling.toString()).version = 10;
  assert.equal(workspaceEditIsCurrent(edit), false);
  assert.equal(edit.replacements.length, 1);
});

test("diagnostic severity maps explicitly across the VS Code and LSP ranges", () => {
  const severities = [
    [1, 0], [2, 1], [3, 2], [4, 3]
  ];
  for (const [lsp, vscode] of severities) {
    const api = numericSeverityApi();
    const projected = lspDiagnosticToVscode(api, {
      range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } },
      severity: lsp,
      message: "message"
    });
    assert.equal(projected.severity, vscode);
    assert.equal(vscodeDiagnosticToLsp(api, projected).severity, lsp);
  }
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

function numericSeverityApi() {
  return {
    ...api,
    DiagnosticSeverity: { Error: 0, Warning: 1, Information: 2, Hint: 3 }
  };
}
