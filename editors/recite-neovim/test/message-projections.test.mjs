import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { projectMessages } from "../scripts/message-projections.mjs";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
const sourcePath = path.join(repositoryRoot, "crates/recite-ui/resources/en-US.ftl");

test("Neovim projection rejects multiline Fluent continuations", async () => {
  const source = await readFile(sourcePath, "utf8");
  const original = "lsp-client-display-name = Recite";
  await assert.rejects(
    projectMessages({ sourceOverride: source.replace(original, `${original}\n  continuation`) }),
    /continuation/,
  );
});

test("Neovim projection rejects selectors and does not truncate them", async () => {
  const source = await readFile(sourcePath, "utf8");
  const original = "lsp-client-display-name = Recite";
  await assert.rejects(
    projectMessages({
      sourceOverride: source.replace(
        original,
        "lsp-client-display-name = { $kind ->\n    [one] one\n   *[other] other\n}",
      ),
    }),
    /unsupported expression/,
  );
});

test("Neovim retains canonical named placeables for its formatter", async () => {
  const { messages } = await projectMessages();
  assert.deepEqual(
    messages.find(([id]) => id === "neovim-callback-failed"),
    ["neovim-callback-failed", "Recite {$kind} callback failed: {$detail}"],
  );
});

test("Neovim projects the full Fluent ASCII variable identifier set", async () => {
  const source = await readFile(sourcePath, "utf8");
  const { messages } = await projectMessages({
    sourceOverride: source.replace(
      "neovim-callback-failed = Recite {$kind} callback failed: {$detail}",
      "neovim-callback-failed = Recite { $Kind-name } callback failed: { $Detail_2 }",
    ),
  });
  assert.deepEqual(
    messages.find(([id]) => id === "neovim-callback-failed"),
    ["neovim-callback-failed", "Recite {$Kind-name} callback failed: {$Detail_2}"],
  );
});
