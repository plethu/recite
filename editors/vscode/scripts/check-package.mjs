import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { readdirSync } from "node:fs";

const packageRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const manifest = JSON.parse(await readFile(path.join(packageRoot, "package.json"), "utf8"));
const languageConfiguration = JSON.parse(
  await readFile(path.join(packageRoot, "language-configuration.json"), "utf8")
);
const projectedMessages = JSON.parse(
  await readFile(path.join(packageRoot, "src", "messages.json"), "utf8")
);
const fluent = await readFile(
  path.resolve(packageRoot, "../../crates/recite-ui/resources/en-US.ftl"), "utf8"
);
const expectedMessageIds = [
  "lsp-client-start-failed",
  "lsp-client-error",
  "lsp-client-exited",
  "lsp-client-restart-scheduled",
  "lsp-client-restart-exhausted"
];
assert(JSON.stringify(Object.keys(projectedMessages).sort()) === JSON.stringify(expectedMessageIds.sort()),
  "VS Code message projection must be complete and contain no unowned IDs");

assert(manifest.name === "recite-vscode", "package name must remain recite-vscode");
assert(manifest.publisher === "plethu", "publisher must remain plethu");
assert(manifest.license === "MIT OR Apache-2.0", "extension license must remain permissive");
assert(manifest.main === "./dist/extension.js", "package must point at the built extension entry point");
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
assert(!manifest.contributes.grammars, "TextMate grammar belongs to REC-97");
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
  const fluentLine = fluent.split("\n").find((line) => line.startsWith(`${id} = `));
  assert(fluentLine, `canonical Fluent message is missing ${id}`);
  assert(fluentLine.slice(fluentLine.indexOf(" = ") + 3).replace("{$detail}", "{0}") === message,
    `VS Code message projection diverges from canonical Fluent message ${id}`);
}

assert(languageConfiguration.comments?.lineComment === "#", "Recite comments must remain # comments");
assert(Array.isArray(languageConfiguration.brackets), "language bracket behavior must be structured");

const source = await readFile(path.join(packageRoot, "src/extension.js"), "utf8");
assert(!source.includes("vscode-languageclient"), "the scaffold must keep its process boundary inspectable");
assert(!source.match(/(?:parse|Parser|tokeniz|compile).*Recite/i),
  "the client must not grow a second Recite semantic implementation");
for (const sourceFile of readdirSync(path.join(packageRoot, "src"), { withFileTypes: true })
  .filter((entry) => entry.isFile() && entry.name.endsWith(".js"))) {
  const syntax = spawnSync(process.execPath, ["--check", path.join(packageRoot, "src", sourceFile.name)], {
    encoding: "utf8"
  });
  assert(syntax.status === 0, `invalid JavaScript in ${sourceFile.name}: ${syntax.stderr}`);
}

console.log("recite-vscode package contract passed");

function assert(condition, message) {
  if (!condition) throw new Error(message);
}
