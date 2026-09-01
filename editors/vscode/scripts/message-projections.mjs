import {
  copyFile,
  lstat,
  mkdir,
  mkdtemp,
  readFile,
  rename,
  rm,
  writeFile
} from "node:fs/promises";
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

export async function generateMessageProjections(packageRoot, options = {}) {
  const fluent = await readFile(
    path.resolve(packageRoot, "../../crates/recite-ui/resources/en-US.ftl"), "utf8"
  );
  const files = renderMessageProjections(projectMessages(fluent));
  const fileSystem = {
    copyFile,
    lstat,
    mkdir,
    mkdtemp,
    rename,
    rm,
    writeFile,
    ...(options.fileSystem ?? {})
  };
  const destinations = Object.entries(files).map(([relative, contents]) => ({
    relative,
    contents,
    target: path.resolve(packageRoot, relative)
  }));
  for (const destination of destinations) {
    await assertSafeDestination(packageRoot, destination.relative, fileSystem);
  }

  let stageRoot;
  const backedUp = [];
  try {
    stageRoot = await fileSystem.mkdtemp(path.join(path.resolve(packageRoot), ".recite-message-projections-"));
    const stageDirectory = path.join(stageRoot, "staged");
    const backupDirectory = path.join(stageRoot, "backups");
    await fileSystem.mkdir(stageDirectory, { recursive: true });
    await fileSystem.mkdir(backupDirectory, { recursive: true });
    for (const destination of destinations) {
      const stagePath = path.join(stageDirectory, destination.relative);
      await fileSystem.mkdir(path.dirname(stagePath), { recursive: true });
      await fileSystem.writeFile(stagePath, destination.contents, "utf8");
      destination.stagePath = stagePath;
      destination.backupPath = path.join(backupDirectory, destination.relative);
      await fileSystem.mkdir(path.dirname(destination.backupPath), { recursive: true });
      await assertSafeAbsent(destination.backupPath, fileSystem);
      await fileSystem.copyFile(destination.target, destination.backupPath);
    }

    for (const destination of destinations) {
      await assertSafeDestination(packageRoot, destination.relative, fileSystem);
    }

    try {
      for (const destination of destinations) {
        await fileSystem.rename(destination.stagePath, destination.target);
        backedUp.push(destination);
      }
    } catch (error) {
      await rollbackProjectionUpdate(backedUp, fileSystem, error);
      throw error;
    }

    for (const destination of destinations) {
      await fileSystem.rm(destination.backupPath, { force: true });
    }
  } finally {
    if (stageRoot) await fileSystem.rm(stageRoot, { recursive: true, force: true });
  }
}

async function assertSafeDestination(packageRoot, relative, fileSystem) {
  const root = path.resolve(packageRoot);
  const target = path.resolve(root, relative);
  const remainder = path.relative(root, target);
  if (!remainder || remainder === ".." || remainder.startsWith(`..${path.sep}`) || path.isAbsolute(remainder)) {
    throw new Error(`refusing projection path outside package root: ${relative}`);
  }

  const rootStat = await fileSystem.lstat(root);
  if (rootStat.isSymbolicLink()) throw new Error(`refusing symlink in projection package root: ${root}`);
  if (!rootStat.isDirectory()) throw new Error(`projection package root is not a directory: ${root}`);

  const components = remainder.split(path.sep);
  let current = root;
  for (const [index, component] of components.entries()) {
    current = path.join(current, component);
    let stat;
    try {
      stat = await fileSystem.lstat(current);
    } catch (error) {
      throw new Error(`projection path is missing: ${path.relative(root, current)}`, { cause: error });
    }
    if (stat.isSymbolicLink()) throw new Error(`refusing symlink in projection path: ${current}`);
    if (index < components.length - 1 && !stat.isDirectory()) {
      throw new Error(`projection parent is not a directory: ${current}`);
    }
    if (index === components.length - 1 && !stat.isFile()) {
      throw new Error(`projection target is not a regular file: ${current}`);
    }
  }
}

async function assertSafeAbsent(file, fileSystem) {
  try {
    const stat = await fileSystem.lstat(file);
    throw new Error(`refusing pre-existing projection backup: ${file} (${stat.isSymbolicLink() ? "symlink" : "path"})`);
  } catch (error) {
    if (error?.code !== "ENOENT") throw error;
  }
}

async function rollbackProjectionUpdate(backedUp, fileSystem, originalError) {
  try {
    for (const destination of backedUp.toReversed()) {
      await fileSystem.rm(destination.target, { force: true });
      await fileSystem.rename(destination.backupPath, destination.target);
    }
  } catch (rollbackError) {
    throw new Error("failed to roll back VS Code message projections", {
      cause: new AggregateError([originalError, rollbackError])
    });
  }
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
