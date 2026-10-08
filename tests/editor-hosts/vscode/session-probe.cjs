const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vscode = require("vscode");

async function change(editor, text, expected) {
  let subscription;
  let timer;
  const ready = new Promise((resolve, reject) => {
    timer = setTimeout(() => reject(new Error("session diagnostic transition timed out")), 10_000);
    subscription = vscode.languages.onDidChangeDiagnostics((event) => {
      if (!event.uris.some(uri => uri.toString() === editor.document.uri.toString())) return;
      const diagnostics = vscode.languages.getDiagnostics(editor.document.uri);
      if (
        expected
          ? diagnostics.some(item => item.message.includes(expected))
          : diagnostics.length === 0
      ) resolve();
    });
  });
  try {
    assert(
      await editor.edit(edit =>
        edit.replace(
          new vscode.Range(
            editor.document.positionAt(0),
            editor.document.positionAt(editor.document.getText().length),
          ),
          text,
        )
      ),
    );
    await ready;
  } finally {
    clearTimeout(timer);
    subscription.dispose();
  }
}

exports.run = async function run(root, cycles) {
  const sources = fs.readdirSync(path.join(root, "src")).filter(name => name.endsWith(".recite"))
    .sort().slice(1, 4);
  const rows = [];
  for (let cycle = 0; cycle < cycles; cycle += 1) {
    const source = path.join(root, "src", sources[cycle % sources.length]);
    const original = fs.readFileSync(source, "utf8");
    const document = await vscode.workspace.openTextDocument(source);
    const editor = await vscode.window.showTextDocument(document);
    const missing = `session_missing_${cycle}`;
    const started = performance.now();
    try {
      await change(editor, original + `\n:: session_probe\n-> ${missing}\n`, missing);
      await change(editor, original, null);
      assert(await document.save());
      const position = document.positionAt(original.indexOf("-> block_") + 3);
      assert(original.includes("-> block_"));
      const completion = await vscode.commands.executeCommand(
        "vscode.executeCompletionItemProvider",
        document.uri,
        position,
      );
      assert(completion?.items?.length > 0);
      await vscode.commands.executeCommand("workbench.action.closeActiveEditor");
      const reopened = await vscode.workspace.openTextDocument(source);
      assert.equal(reopened.getText(), original);
      await vscode.window.showTextDocument(reopened);
      await vscode.commands.executeCommand("workbench.action.closeActiveEditor");
      rows.push({
        cycle,
        elapsedMs: performance.now() - started,
        completionItems: completion.items.length,
      });
    } finally {
      assert.equal(fs.readFileSync(source, "utf8"), original, "session changed the saved fixture");
    }
  }
  return rows;
};
