export function lspRangeToVscode(api, range) {
  return new api.Range(
    new api.Position(range.start.line, range.start.character),
    new api.Position(range.end.line, range.end.character)
  );
}

export function isValidLspRange(range) {
  const start = range?.start;
  const end = range?.end;
  if (!Number.isInteger(start?.line) || start.line < 0 ||
      !Number.isInteger(start?.character) || start.character < 0 ||
      !Number.isInteger(end?.line) || end.line < 0 ||
      !Number.isInteger(end?.character) || end.character < 0) {
    return false;
  }
  return start.line < end.line ||
    start.line === end.line && start.character <= end.character;
}

export function lspLocationToVscode(api, location) {
  if (location?.targetUri && location.targetRange) {
    return new api.Location(api.Uri.parse(location.targetUri), lspRangeToVscode(api, location.targetRange));
  }
  if (!location?.uri || !location.range) return undefined;
  return new api.Location(api.Uri.parse(location.uri), lspRangeToVscode(api, location.range));
}

export function lspDiagnosticToVscode(api, diagnostic) {
  const result = new api.Diagnostic(
    lspRangeToVscode(api, diagnostic.range),
    diagnostic.message ?? "",
    diagnosticSeverity(api, diagnostic.severity)
  );
  if (diagnostic.code !== undefined && diagnostic.code !== null) {
    result.code = typeof diagnostic.code === "object"
      ? String(diagnostic.code.value ?? diagnostic.code.target ?? "")
      : String(diagnostic.code);
  }
  result.source = diagnostic.source ?? "recite";
  return result;
}

export function vscodeDiagnosticToLsp(api, diagnostic) {
  return {
    range: {
      start: { line: diagnostic.range.start.line, character: diagnostic.range.start.character },
      end: { line: diagnostic.range.end.line, character: diagnostic.range.end.character }
    },
    severity: lspSeverity(api, diagnostic.severity),
    code: diagnostic.code,
    source: diagnostic.source,
    message: diagnostic.message
  };
}

export function vscodeCodeActionContextToLsp(api, context) {
  const result = {
    diagnostics: (context?.diagnostics ?? []).map((diagnostic) =>
      vscodeDiagnosticToLsp(api, diagnostic)
    )
  };
  if (context?.only !== undefined) {
    // Some installed VS Code-compatible hosts expose the optional command
    // filter as one CodeActionKind/string instead of the API's array shape.
    // Normalize that boundary before sending the LSP request.
    const only = Array.isArray(context.only) ? context.only : [context.only];
    result.only = only.map(codeActionKindToLsp);
  }
  return result;
}

function codeActionKindToLsp(kind) {
  if (typeof kind === "string") return kind;
  if (typeof kind?.value === "string") return kind.value;
  return kind;
}

export function lspCompletionItems(api, result) {
  const items = Array.isArray(result) ? result : result?.items ?? [];
  return items.map((item) => {
    const completion = new api.CompletionItem(
      item.label ?? "",
      completionKind(api, item.kind)
    );
    if (item.detail !== undefined) completion.detail = item.detail;
    if (item.documentation !== undefined) completion.documentation = markup(api, item.documentation);
    if (item.sortText !== undefined) completion.sortText = item.sortText;
    if (item.filterText !== undefined) completion.filterText = item.filterText;
    if (item.insertText !== undefined) completion.insertText = item.insertText;
    if (item.textEdit?.range && item.textEdit.newText !== undefined) {
      completion.textEdit = new api.TextEdit(
        lspRangeToVscode(api, item.textEdit.range),
        item.textEdit.newText
      );
    }
    return completion;
  });
}

export function lspHoverToVscode(api, result) {
  if (!result) return undefined;
  const contents = Array.isArray(result.contents)
    ? result.contents.map((content) => markup(api, content))
    : [markup(api, result.contents)];
  const range = result.range ? lspRangeToVscode(api, result.range) : undefined;
  return new api.Hover(contents, range);
}

export function lspWorkspaceEditToVscode(api, result, getOpenDocument) {
  if (!result?.documentChanges || !Array.isArray(result.documentChanges)) return undefined;
  const preconditions = [];
  const workspaceEdit = new api.WorkspaceEdit();
  for (const change of result.documentChanges) {
    if (!change?.textDocument?.uri || !Array.isArray(change.edits)) return undefined;
    const version = change.textDocument.version;
    if (!Number.isInteger(version)) return undefined;
    const uri = api.Uri.parse(change.textDocument.uri);
    const document = getOpenDocument(uri);
    if (!document || document.version !== version) return undefined;
    preconditions.push({ document, uri, version });
    for (const edit of change.edits) {
      if (!isValidLspRange(edit?.range) || typeof edit.newText !== "string") return undefined;
      workspaceEdit.replace(uri, lspRangeToVscode(api, edit.range), edit.newText);
    }
  }
  Object.defineProperty(workspaceEdit, "reciteVersionGuard", {
    enumerable: false,
    value: () => workspaceEditStatus(workspaceEdit) === "current"
  });
  Object.defineProperty(workspaceEdit, "reciteVersionPreconditions", {
    enumerable: false,
    value: preconditions
  });
  Object.defineProperty(workspaceEdit, "reciteVersionStatus", {
    enumerable: false,
    value: () => {
      for (const { document: expectedDocument, uri, version } of preconditions) {
        const document = getOpenDocument(uri);
        if (!document) return "document-closed";
        if (document !== expectedDocument) return "document-reopened";
        if (document.version !== version) return "document-stale";
      }
      return "current";
    }
  });
  return workspaceEdit;
}

export function workspaceEditStatus(workspaceEdit) {
  if (workspaceEdit?.reciteVersionStatus) return workspaceEdit.reciteVersionStatus();
  if (workspaceEdit?.reciteVersionGuard && !workspaceEdit.reciteVersionGuard()) {
    return "document-stale";
  }
  return "current";
}

export function workspaceEditIsCurrent(workspaceEdit) {
  return workspaceEditStatus(workspaceEdit) === "current";
}

export function lspCodeActionsToVscode(api, result, getOpenDocument, options = {}) {
  const actions = Array.isArray(result) ? result : [];
  return actions.flatMap((action) => {
    if (!action) return [];
    if (action.title && typeof action.command === "string") {
      return [lspCommandToVscode(action)];
    }
    const codeAction = new api.CodeAction(action.title ?? "", actionKind(api, action.kind));
    if (action.disabled) {
      codeAction.disabled = { reason: action.disabled.reason ?? "" };
      setCodeActionDiagnostics(api, codeAction, action);
      return [codeAction];
    }
    if (action?.edit) {
      const edit = lspWorkspaceEditToVscode(api, action.edit, getOpenDocument);
      if (!edit || !workspaceEditIsCurrent(edit)) return [];
      const command = options.createEditCommand?.(action.title ?? "", edit);
      if (!command) return [];
      // VS Code does not preserve LSP document versions on a native
      // WorkspaceEdit. Keep the edit behind the controller command so its
      // preconditions can be checked again at invocation time.
      codeAction.command = command;
      setCodeActionDiagnostics(api, codeAction, action);
      return [codeAction];
    }
    if (action?.command?.command && action.command.title) {
      return [lspCommandToVscode(action.command)];
    }
    return [];
  });
}

function lspCommandToVscode(command) {
  const result = {
    title: command.title,
    command: command.command
  };
  if (command.arguments !== undefined) result.arguments = command.arguments;
  return result;
}

function setCodeActionDiagnostics(api, codeAction, action) {
  codeAction.isPreferred = action.isPreferred;
  if (Array.isArray(action.diagnostics)) {
    codeAction.diagnostics = action.diagnostics.map((diagnostic) =>
      lspDiagnosticToVscode(api, diagnostic)
    );
  }
}

function diagnosticSeverity(api, severity) {
  switch (severity) {
    case 2: return api.DiagnosticSeverity.Warning;
    case 3: return api.DiagnosticSeverity.Information;
    case 4: return api.DiagnosticSeverity.Hint;
    default: return api.DiagnosticSeverity.Error;
  }
}

function lspSeverity(api, severity) {
  switch (severity) {
    case api.DiagnosticSeverity.Warning: return 2;
    case api.DiagnosticSeverity.Information: return 3;
    case api.DiagnosticSeverity.Hint: return 4;
    case api.DiagnosticSeverity.Error: return 1;
    default: return 1;
  }
}

function completionKind(api, kind) {
  return api.CompletionItemKind?.[completionKindName(kind)] ?? api.CompletionItemKind.Text;
}

function completionKindName(kind) {
  return {
    2: "Method", 3: "Function", 5: "Field", 6: "Variable", 7: "Class", 10: "Property",
    12: "Value", 13: "Enum", 14: "Keyword", 17: "Reference", 18: "File",
    19: "Folder", 21: "Constant", 22: "Struct", 23: "Event", 25: "TypeParameter"
  }[kind] ?? "Text";
}

function actionKind(api, kind) {
  if (kind === "quickfix") return api.CodeActionKind.QuickFix;
  if (kind === "refactor") return api.CodeActionKind.Refactor;
  if (kind === "source") return api.CodeActionKind.Source;
  if (kind === "source.fixAll") return api.CodeActionKind.SourceFixAll ?? "source.fixAll";
  if (kind === "source.organizeImports") {
    return api.CodeActionKind.SourceOrganizeImports ?? "source.organizeImports";
  }
  if (kind?.startsWith("source.fixAll.")) {
    const base = api.CodeActionKind.SourceFixAll ?? "source.fixAll";
    return typeof base.append === "function" ? base.append(kind.slice("source.fixAll".length + 1)) : kind;
  }
  return api.CodeActionKind.Empty;
}

function markup(api, value) {
  if (typeof value === "string") return value;
  if (value?.language && value.value !== undefined) {
    return new api.MarkdownString("```" + value.language + "\n" + value.value + "\n```");
  }
  if (value?.kind === "plaintext" && value.value !== undefined) return value.value;
  if (value?.value !== undefined) return value.value;
  return String(value ?? "");
}
