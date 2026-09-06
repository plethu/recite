const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vscode = require("vscode");

const sleep = (milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds));

async function waitFor(predicate, label, timeout = 30_000) {
  const deadline = Date.now() + timeout;
  for (;;) {
    const value = await predicate();
    if (value) return value;
    if (Date.now() >= deadline) throw new Error(`timed out waiting for ${label}`);
    await sleep(100);
  }
}

function writeResult(value) {
  const destination = process.env.RECITE_HOST_PROBE_RESULT;
  if (!destination) return;
  fs.writeFileSync(destination, `${JSON.stringify(value)}\n`, "utf8");
}

async function installVsix() {
  await vscode.commands.executeCommand(
    "workbench.extensions.installExtension",
    vscode.Uri.file(process.env.RECITE_HOST_PROBE_VSIX),
    { donotSync: true }
  );
  const extensionDirectory = path.join(
    process.env.RECITE_HOST_PROBE_EXTENSIONS,
    "plethu.recite-vscode-0.1.0"
  );
  await waitFor(() => fs.existsSync(path.join(extensionDirectory, "package.json")), "VSIX installation");
  writeResult({
    installed: true,
    extensionDirectory: "isolated-profile",
    profileMarker: writeProfileMarker("install")
  });
}

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

function writeProfileMarker(phase) {
  const destination = process.env.RECITE_HOST_PROBE_PROFILE_MARKER;
  const profileLabel = process.env.RECITE_HOST_PROBE_HOST_LABEL;
  assert(destination, "host profile marker path is configured");
  assert(profileLabel, "host profile marker label is configured");
  const expectedHostProduct = {
    vscode: "Visual Studio Code",
    vscodium: "VSCodium"
  }[profileLabel];
  assert(expectedHostProduct, `unknown host profile marker label: ${profileLabel}`);
  assert.equal(vscode.env.appName, expectedHostProduct,
    "keyboard host profile marker identifies the expected installed host");
  const existing = fs.existsSync(destination)
    ? JSON.parse(fs.readFileSync(destination, "utf8"))
    : undefined;
  if (phase === "install") {
    assert.equal(existing, undefined, "keyboard host profile starts without a marker");
  } else {
    assert(existing, "keyboard host profile retains its install marker");
    assert.equal(existing.event, "clean-profile", "keyboard host profile marker is structured");
    assert.equal(existing.phase, "install", "keyboard host profile marker records installation");
    assert.equal(existing.profileLabel, profileLabel, "keyboard host profile marker is host-specific");
  }
  const marker = {
    event: "clean-profile",
    phase,
    profileLabel,
    hostProduct: vscode.env.appName,
    hostVersion: vscode.version
  };
  fs.writeFileSync(destination, `${JSON.stringify(marker)}\n`, "utf8");
  return marker;
}

async function runKeyboardProbe() {
  const workspace = vscode.Uri.file(process.env.RECITE_HOST_PROBE_WORKSPACE);
  const validUri = vscode.Uri.file(process.env.RECITE_HOST_PROBE_VALID);
  const invalidUri = vscode.Uri.file(process.env.RECITE_HOST_PROBE_INVALID);
  const extension = vscode.extensions.getExtension("plethu.recite-vscode");

  assert(extension, "the installed Recite VSIX is discoverable");
  await vscode.workspace.getConfiguration("recite").update(
    "lsp.path", process.env.RECITE_HOST_PROBE_LSP, vscode.ConfigurationTarget.Workspace
  );
  await vscode.workspace.getConfiguration("recite").update(
    "cli.path", process.env.RECITE_HOST_PROBE_CLI, vscode.ConfigurationTarget.Workspace
  );
  await vscode.workspace.getConfiguration("recite").update(
    "lsp.projectRoot", workspace.fsPath, vscode.ConfigurationTarget.Workspace
  );
  await waitFor(
    () => vscode.workspace.workspaceFolders?.some((folder) => folder.uri.fsPath === workspace.fsPath),
    "keyboard workspace activation"
  );

  writeKeyboardMarker(process.env.RECITE_HOST_PROBE_KEYBOARD_OPEN_READY, {
    event: "open-ready",
    host: vscode.version
  });
  const openedEditor = await waitFor(
    () => vscode.window.activeTextEditor?.document.uri.fsPath === invalidUri.fsPath
      ? vscode.window.activeTextEditor
      : undefined,
    "keyboard source-open active editor"
  );
  assert.equal(openedEditor.document.languageId, "recite",
    "keyboard source-open activates the Recite language");
  await waitFor(() => extension.isActive, "Recite extension activation");
  const openObservation = {
    event: "open",
    activeDocument: openedEditor.document.uri.fsPath,
    language: openedEditor.document.languageId,
    extensionActive: extension.isActive
  };
  writeKeyboardMarker(process.env.RECITE_HOST_PROBE_KEYBOARD_OPEN_RESULT, openObservation);
  const invalid = openedEditor.document;
  const diagnostics = await waitFor(() => vscode.languages.getDiagnostics(invalidUri).filter(
    (diagnostic) => diagnostic.code === "RECITE_PARSE011"
  ), "keyboard diagnostics");
  writeKeyboardMarker(process.env.RECITE_HOST_PROBE_KEYBOARD_READY, {
    event: "ready",
    host: vscode.version,
    language: invalid.languageId,
    diagnostics: diagnostics.length
  });

  const keyResult = await waitFor(
    () => readKeyboardMarker(process.env.RECITE_HOST_PROBE_KEYBOARD_KEY_RESULT),
    "diagnostic keyboard marker"
  );
  assert.equal(keyResult.event, "keyboard-probe", "real keybinding reached the host probe");
  const keyDiagnostic = keyResult.diagnostics.find((diagnostic) => diagnostic.code === "RECITE_PARSE011");
  assert(keyDiagnostic, "keyboard marker preserves the diagnostic code");
  assert.equal(keyDiagnostic.severity, "error", "keyboard marker preserves diagnostic severity");
  assert.equal(keyDiagnostic.start.line, 2, "keyboard marker preserves diagnostic location");
  assert.equal(keyDiagnostic.start.character, 11, "keyboard marker preserves diagnostic start");
  const navigation = {
    activeDocumentMatches: keyResult.navigation?.activeDocument === invalidUri.fsPath,
    selectionMatches: keyResult.navigation?.selection?.line === keyDiagnostic.start.line &&
      keyResult.navigation.selection.character >= keyDiagnostic.start.character &&
      keyResult.navigation.selection.character <= keyDiagnostic.end.character,
    activeDocument: keyResult.navigation?.activeDocument,
    selection: keyResult.navigation?.selection
  };
  assert.equal(navigation.activeDocumentMatches, true,
    "Problems navigation activated the invalid document");
  assert.equal(navigation.selectionMatches, true,
    "Problems navigation selected the diagnostic range");

  const valid = await vscode.workspace.openTextDocument(validUri);
  const editor = await vscode.window.showTextDocument(valid);
  const targetPosition = positionFor(valid.getText(), "-> work", 4);
  editor.selection = new vscode.Selection(targetPosition, targetPosition);
  writeKeyboardMarker(process.env.RECITE_HOST_PROBE_KEYBOARD_RENAME_READY, {
    event: "rename-ready",
    language: valid.languageId,
    cursor: { line: targetPosition.line, character: targetPosition.character }
  });
  await waitFor(() => valid.getText().includes(":: keyboard_done"), "keyboard rename edit");
  writeKeyboardMarker(process.env.RECITE_HOST_PROBE_KEYBOARD_RENAME_RESULT, {
    event: "rename",
    applied: true,
    textContainsRenamedBlock: valid.getText().includes(":: keyboard_done")
  });

  const watchResult = await waitFor(
    () => readKeyboardMarker(process.env.RECITE_HOST_PROBE_KEYBOARD_WATCH_RESULT),
    "keyboard watch stop"
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
    profileMarker: writeProfileMarker("keyboard")
  });
}

async function runHostProbe() {
  const workspace = vscode.Uri.file(process.env.RECITE_HOST_PROBE_WORKSPACE);
  const validUri = vscode.Uri.file(process.env.RECITE_HOST_PROBE_VALID);
  const invalidUri = vscode.Uri.file(process.env.RECITE_HOST_PROBE_INVALID);
  const mainUri = validUri;
  const crossUri = vscode.Uri.file(path.join(workspace.fsPath, "dialogue", "cross-file.recite"));
  const pressureUri = vscode.Uri.file(path.join(workspace.fsPath, "dialogue", "pressure.recite"));
  const overlayUri = vscode.Uri.file(path.join(workspace.fsPath, "scratch", "overlay.recite"));
  const repairUri = vscode.Uri.file(path.join(workspace.fsPath, "dialogue", "repair.recite"));
  const unicodeUri = vscode.Uri.file(path.join(workspace.fsPath, "scratch", "unicode.recite"));
  const assetPath = path.join(workspace.fsPath, "compiled", "dialogue.recitec");
  const fixturePath = path.join(workspace.fsPath, "scratch", "runtime-fixture.toml");
  const invalidFixturePath = path.join(workspace.fsPath, "scratch", "runtime-invalid.toml");
  const extension = vscode.extensions.getExtension("plethu.recite-vscode");

  assert(extension, "the installed Recite VSIX is discoverable");
  await vscode.workspace.getConfiguration("recite").update(
    "lsp.path", process.env.RECITE_HOST_PROBE_LSP, vscode.ConfigurationTarget.Workspace
  );
  await vscode.workspace.getConfiguration("recite").update(
    "cli.path", process.env.RECITE_HOST_PROBE_CLI, vscode.ConfigurationTarget.Workspace
  );
  await vscode.workspace.getConfiguration("recite").update(
    "lsp.projectRoot", workspace.fsPath, vscode.ConfigurationTarget.Workspace
  );

  const diagnosticEvents = [];
  const diagnosticSubscription = vscode.languages.onDidChangeDiagnostics(() => {
    diagnosticEvents.push(true);
  });

  const valid = await vscode.workspace.openTextDocument(mainUri);
  assert.equal(valid.languageId, "recite", ".recite files activate the Recite language");
  await vscode.window.showTextDocument(valid);
  await waitFor(() => extension.isActive, "Recite extension activation");
  await sleep(500);

  const mainText = valid.getText();
  const targetPosition = positionFor(mainText, "-> work", 4);
  const hoverResults = await vscode.commands.executeCommand(
    "vscode.executeHoverProvider", mainUri, targetPosition
  );
  assert(Array.isArray(hoverResults) && hoverResults.length > 0, "hover returns host-projected results");
  assert.equal(hoverResults[0].range.start.line, 6, "hover preserves the canonical source range");
  assert(hoverResults[0].contents.length > 0, "hover preserves structured contents");

  const definitionResults = await vscode.commands.executeCommand(
    "vscode.executeDefinitionProvider", mainUri, targetPosition
  );
  assert(Array.isArray(definitionResults) && definitionResults.length > 0,
    "definition returns host-projected locations");
  assert.equal(definitionResults[0].uri.fsPath, mainUri.fsPath, "same-file definition retains its URI");
  assert.equal(definitionResults[0].range.start.line, 13, "definition preserves the canonical source range");

  const referenceResults = await vscode.commands.executeCommand(
    "vscode.executeReferenceProvider", mainUri, targetPosition
  );
  assert(Array.isArray(referenceResults) && referenceResults.length === 2,
    "references return the canonical location set");
  assert.deepEqual(referenceResults.map((location) => location.range.start.line), [6, 13],
    "references preserve the pinned host's deterministic ordering");

  const pressure = await vscode.workspace.openTextDocument(pressureUri);
  const cross = await vscode.workspace.openTextDocument(crossUri);
  assert.equal(pressure.languageId, "recite", "sibling fixture uses the Recite language");
  assert.equal(cross.languageId, "recite", "cross-file fixture uses the Recite language");
  await vscode.window.showTextDocument(pressure);
  await vscode.window.showTextDocument(cross);
  await sleep(500);
  const crossDefinitionResults = await vscode.commands.executeCommand(
    "vscode.executeDefinitionProvider",
    crossUri,
    positionFor(cross.getText(), "-> dialogue/pressure.recite::le")
  );
  assert(Array.isArray(crossDefinitionResults) && crossDefinitionResults.length > 0,
    "definition resolves the canonical sibling project");
  assert.equal(crossDefinitionResults[0].uri.fsPath, pressureUri.fsPath,
    "cross-file definition preserves the sibling URI");
  assert.equal(crossDefinitionResults[0].range.start.line, 9,
    "cross-file definition preserves the canonical sibling range");

  await vscode.window.showTextDocument(valid);
  const runResult = await vscode.commands.executeCommand("recite.run", {
    asset: assetPath, block: "start", fixture: fixturePath
  });
  const traceResult = await vscode.commands.executeCommand("recite.trace", {
    asset: assetPath, block: "start", fixture: fixturePath
  });
  assert.equal(runResult?.terminal?.status, "success", "run returns a structured success");
  assert.equal(traceResult?.terminal?.status, "success", "trace returns a structured success");
  assert.deepEqual(runResult.terminal.data.trace, traceResult.terminal.data.trace,
    "run and trace preserve shared deterministic trace data");
  assert.equal(runResult.terminal.data.trace.events[0].type, "prompt",
    "run retains the structured prompt event");
  assert(runResult.terminal.data.trace.events.some((event) => event.type === "line"),
    "run retains structured line events");
  const runFailure = await vscode.commands.executeCommand("recite.run", {
    asset: assetPath, block: "start", fixture: invalidFixturePath
  });
  const traceFailure = await vscode.commands.executeCommand("recite.trace", {
    asset: assetPath, block: "start", fixture: invalidFixturePath
  });
  assert.equal(runFailure?.terminal?.event, "command.error", "run exposes a structured failure");
  assert.equal(runFailure?.terminal?.status, "failure", "run failure retains its status");
  assert.equal(traceFailure?.terminal?.event, "command.error", "trace exposes a structured failure");
  assert.equal(traceFailure?.terminal?.status, "failure", "trace failure retains its status");

  const successfulValidation = await vscode.commands.executeCommand("recite.validate");
  assert.equal(successfulValidation?.terminal?.status, "success", "validate reports structured success");
  const completion = await vscode.commands.executeCommand(
    "vscode.executeCompletionItemProvider", mainUri, new vscode.Position(0, 0)
  );
  assert(completion && Array.isArray(completion.items), "completion remains a structured LSP result");

  const invalid = await vscode.workspace.openTextDocument(invalidUri);
  await vscode.window.showTextDocument(invalid);
  await waitFor(
    () => vscode.languages.getDiagnostics(invalidUri).some((diagnostic) => diagnostic.code === "RECITE_PARSE011"),
    "LSP diagnostics"
  );
  const diagnostics = vscode.languages.getDiagnostics(invalidUri);
  assert(diagnosticEvents.length > 0, "the host receives a diagnostic-change notification");

  const failedValidation = await vscode.commands.executeCommand("recite.validate");
  assert.equal(
    failedValidation?.terminal?.status,
    "content_diagnostics",
    "validate exposes typed content-diagnostic failure status"
  );
  assert(
    failedValidation.terminal.data.diagnostics.some((diagnostic) => diagnostic.code === "RECITE_PARSE011"),
    "validate retains the stable diagnostic code"
  );

  const compile = await vscode.commands.executeCommand("recite.compile", {
    output: path.join(workspace.fsPath, "compiled", "from-vscode.recitec")
  });
  assert.equal(compile?.terminal?.status, "content_diagnostics", "compile exposes failure status");
  const extract = await vscode.commands.executeCommand("recite.extract", { output: null });
  assert.equal(extract?.terminal?.status, "content_diagnostics", "extract exposes failure status");

  const watchStarted = await vscode.commands.executeCommand("recite.watch.start");
  assert(watchStarted?.invocationId, "watch returns its structured invocation identifier");
  await sleep(1_500);
  const watchStopped = await vscode.commands.executeCommand("recite.watch.stop");
  assert.equal(watchStopped?.stopped, true, "watch stop completes cooperatively");
  assert.equal(watchStopped?.exitCode, 0, "watch exits cleanly");

  const overlay = await vscode.workspace.openTextDocument(overlayUri);
  const overlayEditor = await vscode.window.showTextDocument(overlay);
  await waitFor(
    () => vscode.languages.getDiagnostics(overlayUri).some((diagnostic) => diagnostic.code === "RECITE_PARSE011"),
    "malformed overlay diagnostics"
  );
  await replaceDocument(overlay, overlayEditor, ":: marker_probe default\n>");
  await waitFor(
    () => vscode.languages.getDiagnostics(overlayUri).length > 0,
    "incomplete overlay recovery diagnostics"
  );
  await replaceDocument(overlay, overlayEditor, mainText);
  await waitFor(
    () => vscode.languages.getDiagnostics(overlayUri).length === 0,
    "recovered overlay diagnostics"
  );

  const unicode = await vscode.workspace.openTextDocument(unicodeUri);
  await vscode.window.showTextDocument(unicode);
  const unicodeDiagnostic = await waitFor(() => vscode.languages.getDiagnostics(unicodeUri).find(
    (diagnostic) => diagnostic.range.start.line === 2
  ), "UTF-16/non-BMP diagnostics");
  assert.equal(unicodeDiagnostic.range.start.character, 13,
    "non-BMP source uses two UTF-16 code units");
  assert.equal(unicodeDiagnostic.range.end.character, 15,
    "CRLF/non-BMP diagnostic end remains UTF-16 stable");

  const repair = await vscode.workspace.openTextDocument(repairUri);
  await vscode.window.showTextDocument(repair);
  const repairDiagnostics = await waitFor(
    () => vscode.languages.getDiagnostics(repairUri).filter((diagnostic) => diagnostic.code === "RECITE_ID001"),
    "stable-ID repair diagnostics"
  );
  const actions = await vscode.commands.executeCommand(
    "vscode.executeCodeActionProvider",
    repairUri,
    new vscode.Range(new vscode.Position(2, 0), new vscode.Position(2, 1)),
    "quickfix"
  );
  assert(Array.isArray(actions), "code actions return host-projected actions");
  const repairAction = actions.find((action) => action.command?.command === "recite.applyCodeAction");
  assert(repairAction?.command?.command === "recite.applyCodeAction",
    "stable-ID repair remains behind the controller command boundary");
  const repairApplied = await vscode.commands.executeCommand(
    repairAction.command.command, ...(repairAction.command.arguments ?? [])
  );
  assert.equal(repairApplied, true, "host applies the guarded stable-ID repair command");
  await waitFor(
    () => /line@[0-9a-f]{20}/u.test(repair.getText()),
    "stable-ID repair edit"
  );

  const renameEditor = await vscode.window.showTextDocument(valid);
  renameEditor.selection = new vscode.Selection(targetPosition, targetPosition);
  const nativeRename = await vscode.commands.executeCommand(
    "vscode.executeDocumentRenameProvider", mainUri, targetPosition, "finished"
  );
  const nativeRenameEntries = typeof nativeRename?.entries === "function"
    ? nativeRename.entries()
    : [];
  assert(nativeRename === undefined || nativeRenameEntries.length === 0,
    "native rename stays unsupported without projected edits");
  // The installed-host API has no supported way to answer the explicit
  // rename prompt without opening a real UI. Exercise the guarded command's
  // missing-document precondition; the pure controller tests cover the
  // versioned edit/apply path without manufacturing keyboard input here.
  await vscode.commands.executeCommand("workbench.action.closeAllEditors");
  const precondition = await vscode.commands.executeCommand("recite.renameBlock");
  assert.equal(precondition, false, "explicit Recite rename rejects a missing active document");

  diagnosticSubscription.dispose();
  writeResult({
    host: vscode.version,
    extensionActive: true,
    language: invalid.languageId,
    hover: true,
    definition: true,
    references: "host-sorted",
    crossFileDefinition: true,
    lspDiagnostics: diagnostics.map((diagnostic) => String(diagnostic.code)),
    diagnosticEvents: diagnosticEvents.length,
    completionCount: completion.items.length,
    validateSuccess: successfulValidation.terminal.status,
    validateFailure: failedValidation.terminal.status,
    compile: compile.terminal.status,
    extract: extract.terminal.status,
    run: runResult.terminal.status,
    trace: traceResult.terminal.status,
    runFailure: runFailure.terminal.status,
    traceFailure: traceFailure.terminal.status,
    watchStopped: watchStopped.stopped,
    watchExitCode: watchStopped.exitCode,
    overlayRecovery: true,
    utf16: "passed",
    codeAction: "stable-id-applied",
    nativeRename: "unsupported",
    nativeRenameShape: nativeRename === undefined ? "undefined" : "empty-workspace-edit",
    rename: "precondition-only"
  });
}

function positionFor(source, needle, offset = needle.length) {
  const byteIndex = source.indexOf(needle);
  assert(byteIndex >= 0, `source is missing ${needle}`);
  const prefix = source.slice(0, byteIndex + offset);
  const lines = prefix.split("\n");
  return new vscode.Position(lines.length - 1, [...lines.at(-1)].reduce(
    (units, scalar) => units + scalar.length,
    0
  ));
}

async function replaceDocument(document, editor, text) {
  const range = new vscode.Range(document.positionAt(0), document.positionAt(document.getText().length));
  assert(await editor.edit((builder) => builder.replace(range, text)), "host applied overlay edit");
}

exports.run = async function run(_testDirectory, callback) {
  try {
    if (process.env.RECITE_HOST_PROBE_INSTALL_ONLY === "1") await installVsix();
    else if (process.env.RECITE_HOST_PROBE_KEYBOARD === "1") await runKeyboardProbe();
    else await runHostProbe();
    callback();
  } catch (error) {
    writeResult({ error: error?.stack ?? String(error) });
    console.error(error?.stack ?? error);
    callback(error);
  }
};
