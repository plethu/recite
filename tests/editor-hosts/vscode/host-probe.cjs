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
  writeResult({ installed: true, extensionDirectory: "isolated-profile" });
}

async function runHostProbe() {
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

  const diagnosticEvents = [];
  const diagnosticSubscription = vscode.languages.onDidChangeDiagnostics(() => {
    diagnosticEvents.push(true);
  });

  const valid = await vscode.workspace.openTextDocument(validUri);
  assert.equal(valid.languageId, "recite", ".recite files activate the Recite language");
  await vscode.window.showTextDocument(valid);
  await waitFor(() => extension.isActive, "Recite extension activation");

  const successfulValidation = await vscode.commands.executeCommand("recite.validate");
  assert.equal(successfulValidation?.terminal?.status, "success", "validate reports structured success");
  const completion = await vscode.commands.executeCommand(
    "vscode.executeCompletionItemProvider", validUri, new vscode.Position(0, 0)
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

  diagnosticSubscription.dispose();
  writeResult({
    host: vscode.version,
    extensionActive: true,
    language: invalid.languageId,
    lspDiagnostics: diagnostics.map((diagnostic) => String(diagnostic.code)),
    diagnosticEvents: diagnosticEvents.length,
    completionCount: completion.items.length,
    validateSuccess: successfulValidation.terminal.status,
    validateFailure: failedValidation.terminal.status,
    compile: compile.terminal.status,
    extract: extract.terminal.status,
    watchStopped: watchStopped.stopped,
    watchExitCode: watchStopped.exitCode
  });
}

exports.run = async function run(_testDirectory, callback) {
  try {
    if (process.env.RECITE_HOST_PROBE_INSTALL_ONLY === "1") await installVsix();
    else await runHostProbe();
    callback();
  } catch (error) {
    writeResult({ error: error?.stack ?? String(error) });
    console.error(error?.stack ?? error);
    callback(error);
  }
};
