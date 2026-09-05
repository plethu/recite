import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import textmate from "vscode-textmate";
import oniguruma from "vscode-oniguruma";

const { Registry, INITIAL, parseRawGrammar } = textmate;
const { OnigScanner, OnigString, loadWASM } = oniguruma;
const packageRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const repositoryRoot = path.resolve(packageRoot, "..", "..");

const grammarSource = await readFile(
  path.join(packageRoot, "syntaxes", "recite.tmLanguage.json"),
  "utf8"
);
const fixtureManifest = JSON.parse(await readFile(
  path.join(repositoryRoot, "fixtures/editor-parity/textmate.json"), "utf8"
));
const snapshots = JSON.parse(await readFile(
  path.join(repositoryRoot, fixtureManifest.token_snapshots), "utf8"
));
const hostileSource = await readFile(path.join(repositoryRoot, fixtureManifest.hostile_fixture), "utf8");
const wasm = await readFile(
  path.join(packageRoot, "node_modules/vscode-oniguruma/release/onig.wasm")
);
await loadWASM(wasm);

const registry = new Registry({
  onigLib: Promise.resolve({
    createOnigScanner: (patterns) => new OnigScanner(patterns),
    createOnigString: (source) => new OnigString(source)
  }),
  loadGrammar: async (scopeName) => scopeName === "source.recite"
    ? parseRawGrammar(grammarSource, "recite.tmLanguage.json")
    : null
});
const grammar = await registry.loadGrammar("source.recite");
assert.ok(grammar, "the pinned tokenizer must load the Recite grammar");

test("checked-in TextMate snapshots execute the pinned tokenizer", () => {
  const fixtureLines = hostileSource.trimEnd().split("\n");
  const snapshotSources = snapshots.cases.map(({ source }) => source);
  assert.deepEqual(fixtureLines, snapshotSources.slice(1));

  for (const { id, source, tokens: expected } of snapshots.cases) {
    assert.deepEqual(tokenize(source), expected, `${id} token snapshot changed`);
  }
});

test("canonical, malformed, and incomplete fixtures remain tokenizable", async () => {
  const paths = [
    ...fixtureManifest.canonical_fixtures,
    fixtureManifest.incomplete_fixture
  ];
  for (const relative of paths) {
    const source = await readFile(path.join(repositoryRoot, relative), "utf8");
    assert.doesNotThrow(() => tokenize(source), relative);
  }
});

function tokenize(source) {
  let state = INITIAL;
  const result = [];
  for (const line of source.replace(/\r\n?/g, "\n").split("\n")) {
    const lineResult = grammar.tokenizeLine(line, state);
    result.push(lineResult.tokens.map((token, index) => [
      line.slice(token.startIndex, lineResult.tokens[index + 1]?.startIndex ?? line.length),
      token.scopes
    ]));
    state = lineResult.ruleStack;
  }
  return source.includes("\n") ? result : result[0];
}
