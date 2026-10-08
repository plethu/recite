const assert = require("node:assert/strict");
const fs = require("node:fs");
const vscode = require("vscode");

function writeKeyboardMarker(destination, value) {
  assert(destination, "keyboard marker path is configured");
  fs.writeFileSync(destination, `${JSON.stringify(value)}\n`, "utf8");
}

function readKeyboardMarker(destination) {
  try {
    return JSON.parse(fs.readFileSync(destination, "utf8"));
  } catch {
    return undefined;
  }
}

exports.runKeyboardProbe = async function runKeyboardProbe(
  { waitFor, writeResult, writeProfileMarker },
) {
  const workspace = vscode.Uri.file(process.env.RECITE_HOST_PROBE_WORKSPACE);
  const validUri = vscode.Uri.file(process.env.RECITE_HOST_PROBE_VALID);
  const invalidUri = vscode.Uri.file(process.env.RECITE_HOST_PROBE_INVALID);
  const extension = vscode.extensions.getExtension("plethu.recite-vscode");

  assert(extension, "the installed Recite VSIX is discoverable");
  await vscode.workspace.getConfiguration("recite").update(
    "lsp.path",
    process.env.RECITE_HOST_PROBE_LSP,
    vscode.ConfigurationTarget.Workspace,
  );
  await vscode.workspace.getConfiguration("recite").update(
    "cli.path",
    process.env.RECITE_HOST_PROBE_CLI,
    vscode.ConfigurationTarget.Workspace,
  );
  await vscode.workspace.getConfiguration("recite").update(
    "lsp.projectRoot",
    workspace.fsPath,
    vscode.ConfigurationTarget.Workspace,
  );
  await waitFor(
    () =>
      vscode.workspace.workspaceFolders?.some((folder) => folder.uri.fsPath === workspace.fsPath),
    "keyboard workspace activation",
  );

  writeKeyboardMarker(process.env.RECITE_HOST_PROBE_KEYBOARD_OPEN_READY, {
    event: "open-ready",
    host: vscode.version,
  });
  const openedEditor = await waitFor(
    () =>
      vscode.window.activeTextEditor?.document.uri.fsPath === invalidUri.fsPath
        ? vscode.window.activeTextEditor
        : undefined,
    "keyboard source-open active editor",
  );
  assert.equal(
    openedEditor.document.languageId,
    "recite",
    "keyboard source-open activates the Recite language",
  );
  await waitFor(() => extension.isActive, "Recite extension activation");
  const openObservation = {
    event: "open",
    activeDocument: openedEditor.document.uri.fsPath,
    language: openedEditor.document.languageId,
    extensionActive: extension.isActive,
  };
  writeKeyboardMarker(process.env.RECITE_HOST_PROBE_KEYBOARD_OPEN_RESULT, openObservation);
  const invalid = openedEditor.document;
  const diagnostics = await waitFor(() =>
    vscode.languages.getDiagnostics(invalidUri).filter(
      (diagnostic) => diagnostic.code === "RECITE_PARSE011",
    ), "keyboard diagnostics");
  writeKeyboardMarker(process.env.RECITE_HOST_PROBE_KEYBOARD_READY, {
    event: "ready",
    host: vscode.version,
    language: invalid.languageId,
    diagnostics: diagnostics.length,
  });

  const keyResult = await waitFor(
    () => readKeyboardMarker(process.env.RECITE_HOST_PROBE_KEYBOARD_KEY_RESULT),
    "diagnostic keyboard marker",
  );
  assert.equal(keyResult.event, "keyboard-probe", "real keybinding reached the host probe");
  const keyDiagnostic = keyResult.diagnostics.find((diagnostic) =>
    diagnostic.code === "RECITE_PARSE011"
  );
  assert(keyDiagnostic, "keyboard marker preserves the diagnostic code");
  assert.equal(keyDiagnostic.severity, "error", "keyboard marker preserves diagnostic severity");
  assert.equal(keyDiagnostic.start.line, 2, "keyboard marker preserves diagnostic location");
  assert.equal(keyDiagnostic.start.character, 11, "keyboard marker preserves diagnostic start");
  const navigation = {
    activeDocumentMatches: keyResult.navigation?.activeDocument === invalidUri.fsPath,
    selectionMatches: keyResult.navigation?.selection?.line === keyDiagnostic.start.line
      && keyResult.navigation.selection.character >= keyDiagnostic.start.character
      && keyResult.navigation.selection.character <= keyDiagnostic.end.character,
    activeDocument: keyResult.navigation?.activeDocument,
    selection: keyResult.navigation?.selection,
  };
  assert.equal(
    navigation.activeDocumentMatches,
    true,
    "Problems navigation activated the invalid document",
  );
  assert.equal(
    navigation.selectionMatches,
    true,
    "Problems navigation selected the diagnostic range",
  );

  const valid = await vscode.workspace.openTextDocument(validUri);
  const editor = await vscode.window.showTextDocument(valid);
  vscode.commands.registerCommand("reciteHostProbe.rename", async () => {
    writeKeyboardMarker(process.env.RECITE_HOST_PROBE_KEYBOARD_RENAME_STARTED, {
      event: "rename-started",
    });
    const result = await vscode.commands.executeCommand("recite.renameBlock");
    writeKeyboardMarker(process.env.RECITE_HOST_PROBE_KEYBOARD_RENAME_COMMAND_RESULT, {
      event: "rename-command-result",
      result,
      activeDocument: vscode.window.activeTextEditor?.document.uri.fsPath,
      textContainsRenamedBlock: valid.getText().includes(":: keyboard_done"),
    });
  });
  const targetPosition = positionFor(valid.getText(), "-> work", 4);
  editor.selection = new vscode.Selection(targetPosition, targetPosition);
  await vscode.commands.executeCommand(
    "vscode.executeCompletionItemProvider",
    validUri,
    targetPosition,
  );
  writeKeyboardMarker(process.env.RECITE_HOST_PROBE_KEYBOARD_RENAME_READY, {
    event: "rename-ready",
    language: valid.languageId,
    cursor: { line: targetPosition.line, character: targetPosition.character },
  });
  await waitFor(() => valid.getText().includes(":: keyboard_done"), "keyboard rename edit");
  writeKeyboardMarker(process.env.RECITE_HOST_PROBE_KEYBOARD_RENAME_RESULT, {
    event: "rename",
    applied: true,
    textContainsRenamedBlock: valid.getText().includes(":: keyboard_done"),
  });

  const watchResult = await waitFor(
    () => readKeyboardMarker(process.env.RECITE_HOST_PROBE_KEYBOARD_WATCH_RESULT),
    "keyboard watch stop",
  );
  assert.equal(watchResult.event, "watch", "keyboard watch marker has the expected event");
  assert.equal(watchResult.started, true, "keyboard start-watch command reached the host");
  assert.equal(watchResult.stopped, true, "keyboard stop-watch command reached the host");
  writeResult({
    host: vscode.version,
    keyboard: "passed",
    keyboardOpen: "activated",
    keyboardOpenUriMatches: openObservation.activeDocument === invalidUri.fsPath,
    keyboardOpenLanguage: openObservation.language,
    keyboardOpenExtensionActive: openObservation.extensionActive,
    keyboardDiagnostics: "navigated",
    keyboardDiagnosticCode: keyDiagnostic.code,
    keyboardDiagnosticSeverity: keyDiagnostic.severity,
    keyboardDiagnosticLine: keyDiagnostic.start.line,
    keyboardNavigation: navigation,
    keyboardRename: "applied",
    keyboardWatch: "stopped",
    profileMarker: writeProfileMarker("keyboard"),
  });
};
