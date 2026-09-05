import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { parseRepresentableMessages } from "../../message-projection-parser.mjs";

const SOURCE = path.resolve(import.meta.dirname, "../../../crates/recite-ui/resources/diagnostics.ftl");
const CONTRACT = path.resolve(import.meta.dirname, "../../../crates/recite-ui/resources/vscode-diagnostic-contract.tsv");
const OUTPUT = path.resolve(import.meta.dirname, "../lua/recite_diagnostics.lua");

function contractEntries(source) {
  const entries = new Map();
  for (const line of source.split(/\r?\n/u)) {
    if (!line || line.startsWith("#")) continue;
    const [id, name = "", type = ""] = line.split("\t");
    const argumentsForId = entries.get(id) ?? [];
    if (name) argumentsForId.push({ name, type });
    entries.set(id, argumentsForId);
  }
  return entries;
}

function diagnosticIds(source) {
  return [...source.matchAll(/(?:^|\n)(diagnostic-[a-z0-9-]+)\s*=\s*[^\n]*/gu)]
    .map((entry) => entry[1])
    .filter((id) => !/(?:-help|-related|-meaning|-cause-\d+|-remediation-\d+)$/u.test(id));
}

function project(source, contractSource) {
  const contracts = contractEntries(contractSource);
  const values = new Map();
  for (const id of diagnosticIds(source)) {
    try {
      const message = parseRepresentableMessages(source, [id], "Neovim diagnostic projection").get(id);
      if (message !== undefined) values.set(id, { template: message, arguments: contracts.get(id) ?? [] });
    } catch (error) {
      if (!/unsupported expression/u.test(error.message)) throw error;
    }
  }
  return { contracts, values };
}

function luaString(value) {
  return JSON.stringify(value);
}

function render({ contracts, values }) {
  const ids = new Set([...contracts.keys(), ...values.keys()]);
  const rows = [...ids].sort().map((id) => {
    const definition = values.get(id);
    const argumentsForId = definition?.arguments ?? contracts.get(id) ?? [];
    const argumentsLua = argumentsForId.map(({ name, type }) =>
      `{ name = ${luaString(name)}, type = ${luaString(type)} }`).join(", ");
    const template = definition ? `, template = ${luaString(definition.template)}` : "";
    return `  [${luaString(id)}] = { arguments = { ${argumentsLua} }${template} },`;
  });
  return `-- Generated from the canonical diagnostic contracts and diagnostics.ftl.\n-- Do not edit; run diagnostic-projections.mjs --update.\nreturn {\n${rows.join("\n")}\n}\n`;
}

export async function renderDiagnosticProjection() {
  const [source, contract] = await Promise.all([readFile(SOURCE, "utf8"), readFile(CONTRACT, "utf8")]);
  return render(project(source, contract));
}

if (process.argv.includes("--update")) {
  await writeFile(OUTPUT, await renderDiagnosticProjection(), "utf8");
} else if (process.argv[1] === new URL(import.meta.url).pathname) {
  const actual = await readFile(OUTPUT, "utf8");
  const expected = await renderDiagnosticProjection();
  if (actual !== expected) throw new Error("Neovim diagnostic projection is stale; run diagnostic-projections.mjs --update");
}
