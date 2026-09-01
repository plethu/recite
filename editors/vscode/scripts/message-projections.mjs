import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";

export const RUNTIME_MESSAGE_IDS = [
  "lsp-client-start-failed",
  "lsp-client-error",
  "lsp-client-exited",
  "lsp-client-restart-scheduled",
  "lsp-client-restart-exhausted",
  "lsp-client-display-name",
  "lsp-client-action-stale",
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

export async function generateMessageProjections(packageRoot) {
  const fluent = await readFile(
    path.resolve(packageRoot, "../../crates/recite-ui/resources/en-US.ftl"), "utf8"
  );
  const projections = projectMessages(fluent);
  await writeFile(
    path.join(packageRoot, "src", "messages.generated.js"),
    `// Generated from crates/recite-ui/resources/en-US.ftl. Do not edit.\nexport default Object.freeze(${JSON.stringify(projections.runtime, null, 2)});\n`,
    "utf8"
  );
  await writeFile(
    path.join(packageRoot, "package.nls.json"),
    `${JSON.stringify(projections.package, null, 2)}\n`,
    "utf8"
  );
}
