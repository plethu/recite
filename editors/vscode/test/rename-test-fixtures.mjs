export function harness() {
  const primary = document("dialogue.recite", 4);
  const sibling = document("sibling.recite", 3);
  const documents = [primary, sibling];
  const applied = [];
  const messages = [];
  const client = { status: "running", request: async () => undefined };
  const api = {
    workspace: {
      isTrusted: true,
      textDocuments: documents,
      applyEdit: async (edit) => { applied.push(edit); return true; }
    },
    window: { activeTextEditor: { document: primary, selection: { active: { line: 1, character: 4 } } } },
    Uri: { parse: (value) => ({ toString: () => value }) },
    Position: class Position {
      constructor(line, character) { this.line = line; this.character = character; }
    },
    Range: class Range {
      constructor(start, end) { this.start = start; this.end = end; }
    },
    WorkspaceEdit: class WorkspaceEdit {
      constructor() { this.replacements = []; }
      replace(uri, editRange, newText) { this.replacements.push({ uri, editRange, newText }); }
    }
  };
  const ui = {
    activeEditor: () => api.window.activeTextEditor,
    documentIsOpen: (document_) => documents.includes(document_),
    chooseRenameName: async () => "renamed",
    commandNotTrusted: () => messages.push("untrusted"),
    renameBusy: () => messages.push("busy"),
    renameDocumentRequired: () => messages.push("document"),
    renameUnavailable: () => messages.push("unavailable"),
    renameInvalid: () => messages.push("invalid"),
    renameStale: () => messages.push("stale"),
    renameApplyFailed: () => messages.push("apply-failed"),
    renameRequestFailed: (detail) => messages.push(["request-failed", detail]),
    serverNotRunning: () => new Error("Recite language server is not running."),
    commandFailure: (detail) => messages.push(["failure", detail])
  };
  return { api, ui, client, primary, sibling, documents, applied, messages };
}

export function document(file, version) {
  return {
    languageId: "recite",
    version,
    uri: { toString: () => `file:///${file}` },
    getText: () => `:: ${file}`
  };
}

export function range() {
  return { start: { line: 1, character: 0 }, end: { line: 1, character: 4 } };
}

export function workspaceEdit(primary, sibling) {
  return {
    documentChanges: [
      {
        textDocument: { uri: primary.uri.toString(), version: primary.version },
        edits: [{ range: range(), newText: "renamed" }]
      },
      {
        textDocument: { uri: sibling.uri.toString(), version: sibling.version },
        edits: [{ range: range(), newText: "renamed" }]
      }
    ]
  };
}
