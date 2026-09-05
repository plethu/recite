import { lstat, readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const packageRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const repositoryRoot = path.resolve(packageRoot, "..", "..");
const grammarRelative = "editors/vscode/syntaxes/recite.tmLanguage.json";
const grammarPath = path.join(repositoryRoot, grammarRelative);
const fixtureManifestPath = path.join(repositoryRoot, "fixtures/editor-parity/textmate.json");

const grammar = await readJson(grammarPath, "TextMate grammar");
const fixtureManifest = await readJson(fixtureManifestPath, "TextMate fixture manifest");
assert(grammar.scopeName === "source.recite", "TextMate grammar must use source.recite");
assert(grammar.name === "Recite", "TextMate grammar must be named Recite");
assert(JSON.stringify(grammar.fileTypes) === JSON.stringify(["recite"]),
  "TextMate grammar must register only the .recite file type");
assert(Array.isArray(grammar.patterns) && grammar.patterns.length > 0,
  "TextMate grammar must have root patterns");
assert(grammar.repository && typeof grammar.repository === "object",
  "TextMate grammar must have a repository");
assert(fixtureManifest.grammar === grammarRelative,
  "TextMate fixture manifest must point at the checked-in grammar");
assert(!grammar.repository["inline-comment"],
  "TextMate grammar must not invent trailing hash comments");
assert(fixtureManifest.hostile_fixture === "fixtures/editor-parity/vscode/textmate-hostile.recite",
  "TextMate fixture manifest must name the hostile fixture");
assert(fixtureManifest.token_snapshots === "fixtures/editor-parity/vscode/textmate-token-snapshots.json",
  "TextMate fixture manifest must name the tokenizer snapshots");

const requiredRules = [
  "comment-line", "block-statement", "line-statement", "choice-statement",
  "effect-statement", "divert-statement", "conditional-statement",
  "plural-statement", "prose-line", "markup-tag", "markup-attribute", "interpolation",
  "invalid-lexical"
];
for (const rule of requiredRules) {
  assert(grammar.repository[rule], `TextMate grammar is missing ${rule}`);
}

const scopes = JSON.stringify(grammar.repository);
for (const scope of [
  "comment.line.number-sign.recite",
  "keyword.control.recite",
  "keyword.control.conditional.recite",
  "punctuation.definition.line.recite",
  "punctuation.definition.choice.recite",
  "punctuation.definition.effect.recite",
  "punctuation.definition.divert.recite",
  "entity.name.section.recite",
  "entity.name.label.recite",
  "constant.other.anchor.recite",
  "variable.other.reference.recite",
  "constant.language.recite",
  "variable.parameter.recite",
  "keyword.operator.assignment.recite",
  "constant.other.symbol.recite",
  "string.quoted.double.recite",
  "constant.numeric.recite",
  "constant.language.boolean.recite",
  "variable.other.runtime.recite",
  "support.function.recite",
  "string.unquoted.prose.recite",
  "entity.name.tag.recite",
  "variable.other.placeholder.recite",
  "invalid.illegal.recite"
]) {
  assert(scopes.includes(scope), `TextMate grammar is missing lexical scope ${scope}`);
}

for (const rule of ["line-statement", "choice-statement"]) {
  const statement = grammar.repository[rule];
  assert(typeof statement.begin === "string", `${rule} must be line-bounded with begin`);
  assert(statement.end === "$", `${rule} must stop at the physical line end`);
  assert(statement.beginCaptures?.["4"]?.name === "punctuation.definition.anchor.recite",
    `${rule} must keep the anchor separator distinct from the label`);
  assert(statement.beginCaptures?.["5"]?.name === "constant.other.anchor.recite",
    `${rule} must keep anchors de-emphasizable as their own scope`);
}

for (const relative of fixtureManifest.canonical_fixtures ?? []) {
  const source = await readRepositoryFile(relative, "canonical fixture");
  assert(/(?:^|\n)[ \t]*(?:::|>|\?|!|->|:if|:else|:match|:case|\|)/.test(source),
    `canonical fixture has no statement marker: ${relative}`);
}
const malformed = await readRepositoryFile(
  "fixtures/recite/invalid/parser_marker_leading_prose.recite", "malformed fixture"
);
assert(/-> East|:if this is a sentence/.test(malformed),
  "malformed fixture no longer exercises marker-leading prose");
const incomplete = await readRepositoryFile(fixtureManifest.incomplete_fixture, "incomplete fixture");
for (const probe of [/unfinished@/, /\{traveller_name$/m, /\? ask_more@/, /->$/m]) {
  assert(probe.test(incomplete), `incomplete fixture is missing recovery probe ${probe}`);
}

console.log("recite-vscode TextMate grammar contract passed");

async function readJson(file, label) {
  await assertRegularFile(file, label);
  try {
    return JSON.parse(await readFile(file, "utf8"));
  } catch (error) {
    throw new Error(`${label} is not valid JSON: ${error.message}`);
  }
}

async function readRepositoryFile(relative, label) {
  assert(typeof relative === "string" && relative.length > 0,
    `${label} path must be a non-empty string`);
  const candidate = path.resolve(repositoryRoot, relative);
  assert(candidate === path.normalize(candidate) &&
    (candidate === repositoryRoot || candidate.startsWith(`${repositoryRoot}${path.sep}`)),
  `${label} path escapes repository: ${relative}`);
  await assertRegularFile(candidate, label);
  return readFile(candidate, "utf8");
}

async function assertRegularFile(file, label) {
  const relative = path.relative(repositoryRoot, file);
  assert(relative && !relative.startsWith(`..${path.sep}`) && !path.isAbsolute(relative),
    `${label} path escapes repository: ${file}`);
  let current = repositoryRoot;
  const components = relative.split(path.sep);
  for (const [index, component] of components.entries()) {
    current = path.join(current, component);
    const stat = await lstat(current);
    assert(!stat.isSymbolicLink(), `${label} must not traverse a symlink: ${current}`);
    if (index < components.length - 1) {
      assert(stat.isDirectory(), `${label} contains a non-directory path component: ${current}`);
    } else {
      assert(stat.isFile(), `${label} must be a regular file: ${file}`);
    }
  }
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}
