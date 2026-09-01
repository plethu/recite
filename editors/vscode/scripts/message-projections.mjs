import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const RUNTIME_MESSAGE_IDS = [
  "lsp-client-start-failed",
  "lsp-client-error",
  "lsp-client-exited",
  "lsp-client-restart-scheduled",
  "lsp-client-restart-exhausted",
  "lsp-client-display-name",
  "lsp-client-action-stale",
  "lsp-client-action-closed",
  "lsp-client-action-reopened",
  "lsp-client-action-expired",
  "lsp-client-action-evicted",
  "lsp-client-action-unknown",
  "lsp-client-action-apply-failed",
  "lsp-client-config-path-invalid",
  "lsp-client-config-args-invalid",
  "lsp-client-config-project-root-invalid",
  "lsp-client-config-project-root-needs-workspace",
  "lsp-client-not-running"
];

export const PACKAGE_MESSAGE_IDS = [
  "lsp-client-display-name",
  "lsp-client-description",
  "lsp-client-untrusted-workspaces-description",
  "lsp-client-configuration-title",
  "lsp-client-configuration-path-description",
  "lsp-client-configuration-args-description",
  "lsp-client-configuration-project-root-description"
];

// These are the extension-owned visible messages. Every source use must go
// through clientMessage and resolve to one of these projected Fluent IDs.
export const SOURCE_MESSAGE_IDS = Object.freeze([...RUNTIME_MESSAGE_IDS]);

export function projectMessages(fluent) {
  const canonical = new Map([...fluent.matchAll(/^([a-z0-9-]+) = ([^\n]*)$/gm)]
    .map((match) => [match[1], match[2]]));
  const projection = (ids, transform = (value) => value) => Object.fromEntries(ids.map((id) => {
    const value = canonical.get(id);
    if (value === undefined) throw new Error(`canonical Fluent message is missing ${id}`);
    return [id, transform(value)];
  }));
  return {
    runtime: projection(RUNTIME_MESSAGE_IDS, (value) => value.replaceAll("{$detail}", "{0}")),
    package: projection(PACKAGE_MESSAGE_IDS)
  };
}

export function renderMessageProjections(projections) {
  return {
    "src/messages.generated.js":
      `// Generated from crates/recite-ui/resources/en-US.ftl. Do not edit.\nexport default Object.freeze(${JSON.stringify(projections.runtime, null, 2)});\n`,
    "package.nls.json": `${JSON.stringify(projections.package, null, 2)}\n`
  };
}

export async function verifyMessageProjections(packageRoot) {
  const fluent = await readFile(
    path.resolve(packageRoot, "../../crates/recite-ui/resources/en-US.ftl"), "utf8"
  );
  const projections = projectMessages(fluent);
  const expectedFiles = renderMessageProjections(projections);
  for (const [relative, expected] of Object.entries(expectedFiles)) {
    const file = path.join(packageRoot, relative);
    let actual;
    try {
      actual = await readFile(file, "utf8");
    } catch (error) {
      throw new Error(`VS Code message projection is missing: ${relative}; run the explicit message update command`, {
        cause: error
      });
    }
    if (actual !== expected) {
      throw new Error(`VS Code message projection is stale: ${relative}; run the explicit message update command`);
    }
  }
  return { fluent, projections };
}

export async function generateMessageProjections(packageRoot) {
  const fluent = await readFile(
    path.resolve(packageRoot, "../../crates/recite-ui/resources/en-US.ftl"), "utf8"
  );
  const files = renderMessageProjections(projectMessages(fluent));
  await Promise.all(Object.entries(files).map(([relative, contents]) =>
    writeFile(path.join(packageRoot, relative), contents, "utf8")
  ));
}

if (path.resolve(process.argv[1] ?? "") === path.resolve(fileURLToPath(import.meta.url))) {
  if (process.argv.length !== 3 || process.argv[2] !== "--update") {
    console.error("Usage: node scripts/message-projections.mjs --update");
    process.exitCode = 2;
  } else {
    const packageRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
    await generateMessageProjections(packageRoot);
    console.log("updated VS Code message projections");
  }
}
