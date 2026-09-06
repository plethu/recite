import test from "node:test";
import assert from "node:assert/strict";
import {
  lspDiagnosticToVscode,
  lspCodeActionsToVscode,
  lspCompletionItems,
  lspWorkspaceEditToVscode,
  vscodeCodeActionContextToLsp,
  vscodeDiagnosticToLsp,
  workspaceEditIsCurrent,
  workspaceEditStatus
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

test("workspace edits require integer versions and an open document", () => {
  const uri = api.Uri.parse("file:///workspace/dialogue.recite");
  const document = { version: 4 };
  const edit = (version) => lspWorkspaceEditToVscode(api, {
    documentChanges: [{
      textDocument: version === "missing"
        ? { uri: uri.toString() }
        : { uri: uri.toString(), version },
      edits: [{
        range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } },
        newText: "#"
      }]
    }]
  }, () => document);

  for (const version of ["missing", null, 4.5, "4"]) {
    assert.equal(edit(version), undefined, `version ${String(version)} must be refused`);
  }

  assert.equal(lspWorkspaceEditToVscode(api, {
    documentChanges: [{
      textDocument: { uri: uri.toString(), version: 4 },
      edits: []
    }]
  }, () => undefined), undefined, "a zero-edit closed-document precondition must be refused");
});

test("workspace edits reject reversed text ranges before host conversion", () => {
  const uri = api.Uri.parse("file:///workspace/dialogue.recite");
  assert.equal(lspWorkspaceEditToVscode(api, {
    documentChanges: [{
      textDocument: { uri: uri.toString(), version: 4 },
      edits: [{
        range: { start: { line: 2, character: 0 }, end: { line: 1, character: 9 } },
        newText: "done"
      }]
    }]
  }, () => ({ version: 4 })), undefined);
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

test("versioned edits require the same open document generation after close and reopen", () => {
  const uri = api.Uri.parse("file:///workspace/dialogue.recite");
  const firstGeneration = { version: 4 };
  const reopenedGeneration = { version: 4 };
  let openDocument = firstGeneration;
  const edit = lspWorkspaceEditToVscode(api, {
    documentChanges: [{
      textDocument: { uri: uri.toString(), version: 4 },
      edits: []
    }]
  }, () => openDocument);

  assert.equal(workspaceEditIsCurrent(edit), true);
  openDocument = undefined;
  assert.equal(workspaceEditIsCurrent(edit), false);
  openDocument = reopenedGeneration;
  assert.equal(workspaceEditIsCurrent(edit), false);
  openDocument = firstGeneration;
  assert.equal(workspaceEditIsCurrent(edit), true);
});

test("versioned workspace edits expose distinct document lifecycle outcomes", () => {
  const uri = api.Uri.parse("file:///workspace/dialogue.recite");
  const firstGeneration = { version: 4 };
  const reopenedGeneration = { version: 4 };
  let openDocument = firstGeneration;
  const edit = lspWorkspaceEditToVscode(api, {
    documentChanges: [{
      textDocument: { uri: uri.toString(), version: 4 },
      edits: []
    }]
  }, () => openDocument);

  assert.equal(workspaceEditStatus(edit), "current");
  firstGeneration.version = 5;
  assert.equal(workspaceEditStatus(edit), "document-stale");
  firstGeneration.version = 4;
  openDocument = undefined;
  assert.equal(workspaceEditStatus(edit), "document-closed");
  openDocument = reopenedGeneration;
  assert.equal(workspaceEditStatus(edit), "document-reopened");
});

test("editable code actions are projected as controller-owned commands", () => {
  const uri = api.Uri.parse("file:///workspace/dialogue.recite");
  const document = { version: 4 };
  const command = { title: "Apply fix", command: "recite.applyCodeAction", arguments: ["1"] };
  const actions = lspCodeActionsToVscode(api, [{
    title: "Apply fix",
    kind: "quickfix",
    edit: {
      documentChanges: [{
        textDocument: { uri: uri.toString(), version: 4 },
        edits: [{
          range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } },
          newText: "#"
        }]
      }]
    }
  }], () => document, {
    createEditCommand: () => command
  });

  assert.equal(actions.length, 1);
  assert.equal(actions[0].edit, undefined);
  assert.deepEqual(actions[0].command, command);
});

test("disabled and nested source-fix-all actions preserve their LSP shape", () => {
  const actions = lspCodeActionsToVscode(api, [
    {
      title: "Unavailable fix",
      kind: "quickfix",
      disabled: { reason: "requires a project schema" }
    },
    {
      title: "Fix all Recite files",
      kind: "source.fixAll.recite",
      edit: { documentChanges: [] }
    }
  ], () => undefined, {
    createEditCommand: () => ({ title: "Fix all Recite files", command: "recite.applyCodeAction", arguments: ["1"] })
  });

  assert.equal(actions.length, 2);
  assert.deepEqual(actions[0].disabled, { reason: "requires a project schema" });
  assert.equal(actions[1].kind, "source.fixAll.recite");
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

test("code-action context projects kind values and preserves only semantics", () => {
  const requested = vscodeCodeActionContextToLsp(api, {
    diagnostics: [],
    only: [api.CodeActionKind.QuickFix, { value: "source.fixAll" }, "custom.recite"]
  });
  assert.deepEqual(requested.only, ["quickfix", "source.fixAll", "custom.recite"]);

  const absent = vscodeCodeActionContextToLsp(api, { diagnostics: [] });
  assert.equal(Object.hasOwn(absent, "only"), false);
  assert.deepEqual(vscodeCodeActionContextToLsp(api, { diagnostics: [], only: [] }).only, []);
  assert.deepEqual(vscodeCodeActionContextToLsp(api, {
    diagnostics: [], only: "quickfix"
  }).only, ["quickfix"]);
});

test("completion projection preserves field identity and filter text", () => {
  const field = Symbol("Field");
  const text = Symbol("Text");
  const completionApi = {
    ...api,
    CompletionItem: class CompletionItem {
      constructor(label, kind) {
        this.label = label;
        this.kind = kind;
      }
    },
    CompletionItemKind: { Field: field, Text: text }
  };
  const [completion] = lspCompletionItems(completionApi, [{
    label: "portrait",
    kind: 5,
    filterText: "por"
  }]);

  assert.equal(completion.kind, field);
  assert.equal(completion.label, "portrait");
  assert.equal(completion.filterText, "por");
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
  },
  CodeAction: class CodeAction {
    constructor(title, kind) { this.title = title; this.kind = kind; }
  },
  CodeActionKind: {
    QuickFix: "quickfix",
    Refactor: "refactor",
    Source: "source",
    SourceFixAll: { append: (value) => `source.fixAll.${value}` },
    SourceOrganizeImports: "source.organizeImports",
    Empty: ""
  }
};

function numericSeverityApi() {
  return {
    ...api,
    DiagnosticSeverity: { Error: 0, Warning: 1, Information: 2, Hint: 3 }
  };
}
