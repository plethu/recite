const assert = require("node:assert/strict");
const crypto = require("node:crypto");
const fs = require("node:fs");
const path = require("node:path");
const vscode = require("vscode");

async function measure() {
  const root = process.env.RECITE_PERF_ROOT;
  const configuration = vscode.workspace.getConfiguration("recite");
  await configuration.update(
    "lsp.path",
    process.env.RECITE_PERF_BINARY,
    vscode.ConfigurationTarget.Workspace,
  );
  await configuration.update("lsp.projectRoot", root, vscode.ConfigurationTarget.Workspace);
  const source =
    fs.readdirSync(path.join(root, "src")).filter((name) => name.endsWith(".recite")).sort()[0];
  const document = await vscode.workspace.openTextDocument(
    vscode.Uri.file(path.join(root, "src", source)),
  );
  const editor = await vscode.window.showTextDocument(document);
  const extension = vscode.extensions.getExtension("plethu.recite-vscode");
  assert(extension);
  await extension.activate();
  const renderer = process.env.RECITE_PERF_CDP_PORT_FILE
    ? await require("./render-probe.cjs").connect(process.env.RECITE_PERF_CDP_PORT_FILE)
    : null;
  const original = document.getText();
  const reference = document.positionAt(original.indexOf("-> block_") + 3);
  assert(original.includes("-> block_"));
  const report = {
    host: vscode.env.appName,
    version: vscode.version,
    platform: process.platform,
    architecture: process.arch,
    project: root,
    binarySha256: crypto.createHash("sha256").update(
      fs.readFileSync(process.env.RECITE_PERF_BINARY),
    ).digest("hex"),
    sourceSha256: crypto.createHash("sha256").update(original).digest("hex"),
    extensionVersion: extension.packageJSON.version,
    sourceBytes: Buffer.byteLength(original),
    editShape: "replace appended block only",
    screenRenderingMeasured: Boolean(renderer),
    physicalDisplayMeasured: false,
    semanticEditToDiagnosticStoreMs: [],
    completionProviderMs: [],
    visibleDecorationFrameMs: [],
    compositorFrameCaptureMs: [],
    frameStates: [],
    renderingMethod: renderer
      ? "visible decoration plus two animation frames and compositor surface capture; includes capture overhead"
      : null,
  };
  for (let index = 0; index < 23; index += 1) {
    const missing = `host_missing_${index}`;
    const broken = index % 2 === 0;
    let subscription;
    let timer;
    const started = performance.now();
    const published = new Promise((resolve, reject) => {
      timer = setTimeout(() => reject(new Error("diagnostic-store update timed out")), 30_000);
      subscription = vscode.languages.onDidChangeDiagnostics((event) => {
        if (!event.uris.some((uri) => uri.toString() === document.uri.toString())) return;
        const diagnostics = vscode.languages.getDiagnostics(document.uri);
        if (
          broken
            ? diagnostics.some((item) => item.message.includes(missing))
            : diagnostics.length === 0
        ) {
          resolve(performance.now() - started);
        }
      });
    });
    try {
      assert(
        await editor.edit((edit) =>
          edit.replace(
            new vscode.Range(
              document.positionAt(original.length),
              document.positionAt(document.getText().length),
            ),
            `\n:: host_probe\n-> ${broken ? missing : "END"}\n`,
          )
        ),
      );
      if (renderer) {
        const tail = document.positionAt(document.getText().length - 1);
        editor.selection = new vscode.Selection(tail, tail);
        editor.revealRange(new vscode.Range(tail, tail), vscode.TextEditorRevealType.InCenter);
      }
      const elapsed = await published;
      if (index >= 2) report.semanticEditToDiagnosticStoreMs.push(elapsed);
      if (renderer) {
        const destination = index === 2 || index === 3
          ? `${process.env.RECITE_PERF_OUTPUT}.${broken ? "error" : "clear"}.png`
          : null;
        const state = await renderer.capture(missing, broken, destination);
        if (index >= 2) {
          report.visibleDecorationFrameMs.push(state.readyAt - started);
          report.compositorFrameCaptureMs.push(performance.now() - started);
          report.frameStates.push({ broken, marks: state.marks });
        }
      }
    } finally {
      clearTimeout(timer);
      subscription.dispose();
    }
    const queryStarted = performance.now();
    const completions = await vscode.commands.executeCommand(
      "vscode.executeCompletionItemProvider",
      document.uri,
      reference,
    );
    assert(completions?.items?.length > 0, "completion provider returned no items");
    if (index >= 2) report.completionProviderMs.push(performance.now() - queryStarted);
  }
  renderer?.close();
  if (process.env.RECITE_PERF_SESSION_CYCLES) {
    report.sessionCycles = await require("./session-probe.cjs").run(
      root,
      Number(process.env.RECITE_PERF_SESSION_CYCLES),
    );
  }
  assert.equal(
    fs.readFileSync(document.uri.fsPath, "utf8"),
    original,
    "probe changed a saved source",
  );
  fs.writeFileSync(process.env.RECITE_PERF_OUTPUT, `${JSON.stringify(report, null, 2)}\n`);
}

exports.run = async function run(_directory, callback) {
  try {
    await measure();
    callback();
  } catch (error) {
    console.error(error);
    callback(error);
  }
};
