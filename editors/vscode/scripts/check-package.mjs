import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import {
  PACKAGE_MESSAGE_IDS,
  projectRuntimeMessage,
  RUNTIME_MESSAGE_IDS,
  SOURCE_MESSAGE_IDS,
  verifyMessageProjections
} from "./message-projections.mjs";
import { parseRepresentableMessages } from "../../message-projection-parser.mjs";
import { listSourceModules } from "./source-files.mjs";
import { assertSafeTree } from "./safety.mjs";
import { assertUiBoundary } from "./ui-boundary.mjs";

const packageRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const manifest = JSON.parse(await readFile(path.join(packageRoot, "package.json"), "utf8"));
const languageConfiguration = JSON.parse(
  await readFile(path.join(packageRoot, "language-configuration.json"), "utf8")
);
assertSafeTree(path.join(packageRoot, "syntaxes"), "extension syntax grammars");
const grammarPath = path.join(packageRoot, "syntaxes", "recite.tmLanguage.json");
const grammar = JSON.parse(await readFile(grammarPath, "utf8"));
const { fluent, projections } = await verifyMessageProjections(packageRoot);
const projectedMessages = projections.runtime;
const packageMessages = projections.package;
const canonicalMessages = parseRepresentableMessages(
  fluent,
  [...RUNTIME_MESSAGE_IDS, ...PACKAGE_MESSAGE_IDS],
  "VS Code package contract"
);
assertSameKeys(Object.keys(projectedMessages), RUNTIME_MESSAGE_IDS,
  "VS Code runtime message projection");
assertSameKeys(Object.keys(packageMessages), PACKAGE_MESSAGE_IDS,
  "VS Code package message projection");

assert(manifest.name === "recite-vscode", "package name must remain recite-vscode");
assert(manifest.publisher === "plethu", "publisher must remain plethu");
assert(manifest.license === "MIT OR Apache-2.0", "extension license must remain permissive");
assert(manifest.main === "./dist/extension.cjs", "package must point at the CommonJS extension entry point");
assert(manifest.engines?.vscode, "the VS Code engine range is required");
assert(manifest.activationEvents?.includes("onLanguage:recite"), "activation must be tied to Recite files");
const trust = manifest.capabilities?.untrustedWorkspaces;
assert(trust?.supported === true, "restricted workspaces must be supported without starting a server");
assert(JSON.stringify(trust.restrictedConfigurations?.slice().sort()) === JSON.stringify([
  "recite.lsp.args", "recite.lsp.path", "recite.lsp.projectRoot"
]), "all process-affecting settings must be restricted in untrusted workspaces");

const languages = manifest.contributes?.languages ?? [];
const reciteLanguage = languages.find((language) => language.id === "recite");
assert(reciteLanguage, "the Recite language contribution is required");
assert(reciteLanguage.extensions?.length === 1 && reciteLanguage.extensions[0] === ".recite",
  "only .recite source files may activate the language contribution");
assert(reciteLanguage.configuration === "./language-configuration.json",
  "language editing behavior must be explicit and local");
const grammars = manifest.contributes?.grammars ?? [];
assert(grammars.length === 1, "the Recite TextMate grammar contribution is required");
assert(grammars[0].language === "recite" && grammars[0].scopeName === "source.recite" &&
  grammars[0].path === "./syntaxes/recite.tmLanguage.json",
"the Recite TextMate grammar contribution must use the checked-in source grammar");
assert(Array.isArray(manifest.files) && manifest.files.includes("syntaxes/**"),
  "the TextMate grammar must be included in the extension package");
assert(grammar.name === "Recite" && grammar.scopeName === "source.recite" &&
  Array.isArray(grammar.fileTypes) && grammar.fileTypes.length === 1 && grammar.fileTypes[0] === "recite",
"the TextMate grammar must declare the Recite source scope and .recite file type");
assert(grammar.repository && typeof grammar.repository === "object",
  "the TextMate grammar repository is required");
for (const rule of [
  "comment-line", "block-statement", "line-statement", "choice-statement",
  "effect-statement", "divert-statement", "conditional-statement", "plural-statement",
  "prose-line", "markup-tag", "interpolation", "invalid-lexical"
]) {
  assert(grammar.repository[rule], `TextMate grammar is missing lexical rule ${rule}`);
}
assert(!manifest.contributes.commands, "commands belong to REC-53");
assert(!manifest.contributes.menus, "menus belong to REC-53");
assert(!manifest.contributes.tasks, "tasks belong to REC-53");

const properties = manifest.contributes.configuration?.properties ?? {};
assert(properties["recite.lsp.path"]?.type === "string", "server path must be a string setting");
assert(properties["recite.lsp.args"]?.type === "array" &&
  properties["recite.lsp.args"].items?.type === "string",
"server arguments must be an explicit string array");
assert(properties["recite.lsp.projectRoot"]?.type === "string",
  "project root must be a string setting");

for (const [id, message] of Object.entries(projectedMessages)) {
  assert(canonicalMessages.has(id), `canonical Fluent message is missing ${id}`);
  assert(projectRuntimeMessage(id, canonicalMessages.get(id)) === message,
    `VS Code message projection diverges from canonical Fluent message ${id}`);
}
for (const [id, message] of Object.entries(packageMessages)) {
  assert(canonicalMessages.has(id), `canonical Fluent message is missing ${id}`);
  assert(canonicalMessages.get(id) === message,
    `VS Code package projection diverges from canonical Fluent message ${id}`);
}

for (const value of localizableManifestValues(manifest)) {
  assert(/^%[^%]+%$/.test(value), `visible package text must use a message projection: ${value}`);
  const id = value.slice(1, -1);
  assert(Object.hasOwn(packageMessages, id), `package message projection is missing ${id}`);
}

assert(languageConfiguration.comments?.lineComment === "#", "Recite comments must remain # comments");
assert(Array.isArray(languageConfiguration.brackets), "language bracket behavior must be structured");

const sourceRoot = path.join(packageRoot, "src");
assertSafeTree(sourceRoot, "extension source");
const source = await readFile(path.join(sourceRoot, "extension.js"), "utf8");
const entrySource = await readFile(path.join(sourceRoot, "extension.cjs"), "utf8");
assert(entrySource.includes('require("vscode")'),
  "the CommonJS extension entry must acquire the VS Code host API");
assert(!source.match(/(?:from|import\s*\()\s*["']vscode["']/),
  "the ESM implementation must receive the VS Code host API explicitly");
assert(source.includes("activateWithVscode"),
  "the ESM implementation must expose the explicit VS Code activation boundary");
assert(!source.includes("vscode-languageclient"), "the scaffold must keep its process boundary inspectable");
assert(!source.match(/(?:parse|Parser|tokeniz|compile).*Recite/i),
  "the client must not grow a second Recite semantic implementation");
const sourceFiles = listSourceModules(sourceRoot);
assert(sourceFiles.some(({ relativePath }) => relativePath === "extension.cjs"),
  "the CommonJS extension entry shim is required");
const sourceContents = await Promise.all(sourceFiles.map(async ({ relativePath, absolutePath }) => [
  relativePath,
  await readFile(absolutePath, "utf8")
]));
assertUiBoundary(sourceContents, SOURCE_MESSAGE_IDS, projectedMessages);
for (const { relativePath, absolutePath } of sourceFiles) {
  const syntax = spawnSync(process.execPath, ["--check", absolutePath], {
    encoding: "utf8"
  });
  assert(syntax.status === 0, `invalid JavaScript in ${relativePath}: ${syntax.stderr}`);
}

console.log("recite-vscode package contract passed");

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function assertSameKeys(actual, expected, label) {
  assert(JSON.stringify(actual.slice().sort()) === JSON.stringify(expected.slice().sort()),
    `${label} must be complete and contain no unowned IDs`);
}

function localizableManifestValues(packageManifest) {
  return [
    packageManifest.displayName,
    packageManifest.description,
    packageManifest.capabilities?.untrustedWorkspaces?.description,
    packageManifest.contributes?.configuration?.title,
    ...Object.values(packageManifest.contributes?.configuration?.properties ?? {})
      .map((property) => property.description)
  ].filter((value) => value !== undefined);
}
