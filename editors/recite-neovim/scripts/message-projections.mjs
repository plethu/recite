import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { parseRepresentableMessages } from "../../message-projection-parser.mjs";

const scriptRoot = path.dirname(fileURLToPath(import.meta.url));
const packageRoot = path.resolve(scriptRoot, "..");
const repositoryRoot = path.resolve(packageRoot, "../..");
const inventoryPath = path.join(repositoryRoot, "crates", "recite-ui", "resources", "inventory.toml");

export async function projectMessages({ sourceOverride } = {}) {
  const inventory = await readFile(inventoryPath, "utf8");
  const projection = inventory.match(/\[projections\.neovim\]([\s\S]*?)(?=\n\[|$)/)?.[1];
  if (!projection) throw new Error("Neovim UI projection is missing from the canonical inventory");

  const sourceResource = projection.match(/(?:^|\n)source_resource\s*=\s*"([^"]+)"/)?.[1];
  const output = projection.match(/(?:^|\n)output\s*=\s*"([^"]+)"/)?.[1];
  const idsSource = projection.match(/(?:^|\n)ids\s*=\s*\[([\s\S]*?)\]/)?.[1];
  const ids = [...(idsSource ?? "").matchAll(/"([a-z0-9-]+)"/g)].map((match) => match[1]);
  if (!sourceResource || !output || ids.length === 0) {
    throw new Error("Neovim UI projection must declare a source, output, and IDs");
  }

  const source = sourceOverride ?? await readFile(
    path.join(repositoryRoot, "crates", "recite-ui", "resources", sourceResource), "utf8"
  );
  const canonical = parseRepresentableMessages(source, ids, "Neovim");
  const messages = ids.map((id) => {
    const value = canonical.get(id);
    if (value === undefined) throw new Error(`canonical Fluent message is missing ${id}`);
    return [id, value];
  });
  const destination = path.join(repositoryRoot, output);
  return { destination, ids, messages };
}

async function assertCallsites(ids) {
  const files = [
    path.join(packageRoot, "lua", "recite.lua"),
    path.join(packageRoot, "lua", "recite", "health.lua"),
  ];
  const known = new Set(ids);
  for (const file of files) {
    const source = await readFile(file, "utf8");
    for (const match of source.matchAll(/messages\.format\("([a-z0-9-]+)"/g)) {
      if (!known.has(match[1])) {
        throw new Error(`${path.relative(repositoryRoot, file)} uses an undeclared Neovim UI message ${match[1]}`);
      }
    }
    if (/\bhealth\.(?:start|ok|error|warn|info)\(\s*["']/.test(source)
      || /vim\.notify\(\s*["']/.test(source)
      || /\bdesc\s*=\s*["']/.test(source)) {
      throw new Error(`${path.relative(repositoryRoot, file)} contains a hard-coded Recite-owned UI string`);
    }
  }
}

export function renderMessages(messages) {
  const lines = [
    "-- Generated from crates/recite-ui/resources/en-US.ftl via the canonical inventory.",
    "-- Do not edit. Run editors/recite-neovim/scripts/message-projections.mjs to regenerate.",
    "local templates = {",
  ];
  for (const [id, value] of messages) {
    lines.push(`  [${JSON.stringify(id)}] = ${JSON.stringify(value)},`);
  }
  lines.push(
    "}",
    "",
    "local M = {}",
    "",
    "function M.format(id, arguments)",
    "  local template = templates[id]",
    "  if template == nil then",
    "    error(\"unknown Recite UI message: \" .. tostring(id), 2)",
    "  end",
    "  arguments = arguments or {}",
    "  return (template:gsub(\"{%$([%w_]+)}\", function(name)",
    "    local value = arguments[name]",
    "    if value == nil then",
    "      error(\"missing argument for Recite UI message \" .. id .. \".$\" .. name, 2)",
    "    end",
    "    return tostring(value)",
    "  end))",
    "end",
    "",
    "return M",
    "",
  );
  return `${lines.join("\n")}`;
}

if (path.resolve(process.argv[1] ?? "") === path.resolve(fileURLToPath(import.meta.url))) {
  const { destination, ids, messages } = await projectMessages();
  await assertCallsites(ids);
  const generated = renderMessages(messages);
  if (process.argv.includes("--check")) {
    const current = await readFile(destination, "utf8").catch(() => null);
    if (current !== generated) {
      throw new Error(`${path.relative(repositoryRoot, destination)} is stale; run the message projection generator`);
    }
  } else {
    await writeFile(destination, generated, "utf8");
  }
}
