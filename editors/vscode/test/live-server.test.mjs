import test from "node:test";
import assert from "node:assert/strict";
import { access, mkdtemp, writeFile, readFile, rm } from "node:fs/promises";
import path from "node:path";
import os from "node:os";
import { fileURLToPath, pathToFileURL } from "node:url";
import { ReciteLanguageClient } from "../src/lsp-client.js";

const packageRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const serverBinary = process.env.RECITE_LSP_BIN ?? path.resolve(packageRoot, "../../target/debug/recite-lsp");
const coreFixturePath = path.resolve(packageRoot, "../../fixtures/recite/valid/core_language_spike.recite");
const schemaFixturePath = path.resolve(packageRoot, "../../fixtures/schema/valid/generated_manifest.json");

test("built recite-lsp handles effective root, UTF-16 diagnostics, watcher refresh, and shutdown", {
  skip: !(await exists(serverBinary)),
  timeout: 15_000
}, async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "recite-vscode-live-"));
  const sourcePath = path.join(root, "dialogue.recite");
  const siblingPath = path.join(root, "pressure.recite");
  const schemaPath = path.join(root, "schema.json");
  const source = (await readFile(coreFixturePath, "utf8"))
    .replace("-> work", "-> pressure.recite::letters")
    .replace(
      "! deferred advance_thread(start, asked)\n-> END",
      "! deferred advance_thread(start, asked)\n> metadata_line@32a122c362c13a9fba4e por\n  Metadata.\n-> END"
    );
  const sibling = ":: letters\n> line@88990011223344556677\n  Letters.\n-> END\n";
  await writeFile(sourcePath, source);
  await writeFile(schemaPath, await readFile(schemaFixturePath, "utf8"));
  const sourceUri = pathToFileURL(sourcePath).toString();
  const siblingUri = pathToFileURL(siblingPath).toString();
  const diagnostics = [];
  const registrations = [];
  const client = new ReciteLanguageClient({
    command: serverBinary,
    args: [],
    cwd: root,
    onRegisterCapability: (params) => registrations.push(params)
  });
  client.on("notification", (method, params) => {
    if (method === "textDocument/publishDiagnostics" && params.uri === sourceUri) diagnostics.push(params);
  });

  try {
    await client.start({
      processId: process.pid,
      rootUri: pathToFileURL(root).toString(),
      workspaceFolders: [{ uri: pathToFileURL(root).toString(), name: "effective-root" }],
      initializationOptions: { schema: schemaPath },
      capabilities: {
        general: { positionEncodings: ["utf-16"] },
        workspace: { configuration: true, didChangeWatchedFiles: { dynamicRegistration: true } }
      }
    });
    await waitFor(() => registrations.some((params) => params.registrations?.some(
      (registration) => registration.method === "workspace/didChangeWatchedFiles"
    )));

    client.notify("textDocument/didOpen", {
      textDocument: { uri: sourceUri, languageId: "recite", version: 1, text: source }
    });
    await waitFor(() => diagnostics.length > 0);
    assert.equal(diagnostics.at(-1).version, 1);
    assert.ok(diagnostics.at(-1).diagnostics.every((diagnostic) =>
      Number.isInteger(diagnostic.range.start.character)
    ));

    const completion = await client.request("textDocument/completion", {
      textDocument: { uri: sourceUri },
      position: positionAfter(source, "> metadata_line@32a122c362c13a9fba4e por")
    });
    assert.ok(Array.isArray(completion));
    const portrait = completion.find((item) => item.label === "portrait");
    assert.deepEqual({
      label: portrait?.label,
      kind: portrait?.kind,
      filterText: portrait?.filterText
    }, {
      label: "portrait",
      kind: 5,
      filterText: undefined
    });

    await writeFile(siblingPath, sibling);
    const beforeWatch = diagnostics.length;
    client.notify("workspace/didChangeWatchedFiles", {
      changes: [{ type: 1, uri: siblingUri }]
    });
    await waitFor(() => diagnostics.length > beforeWatch);
    const definition = await client.request("textDocument/definition", {
      textDocument: { uri: sourceUri },
      position: { line: source.split("\n").findIndex((line) => line.includes("pressure")), character: 24 }
    });
    assert.equal(definition?.uri, siblingUri);
  } finally {
    await client.stop();
    await rm(root, { recursive: true, force: true });
  }
  assert.equal(client.status, "stopped");
});

async function exists(file) {
  try {
    await access(file);
    return true;
  } catch {
    return false;
  }
}

async function waitFor(predicate) {
  const deadline = Date.now() + 5_000;
  while (!predicate()) {
    if (Date.now() >= deadline) throw new Error("timed out waiting for live recite-lsp evidence");
    await new Promise((resolve) => setTimeout(resolve, 20));
  }
}

function positionAfter(source, needle) {
  const end = source.indexOf(needle) + needle.length;
  const before = source.slice(0, end);
  const lines = before.split("\n");
  return { line: lines.length - 1, character: lines.at(-1).length };
}
