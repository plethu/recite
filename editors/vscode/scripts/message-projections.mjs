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
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const packageRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const inventoryPath = path.resolve(packageRoot, "../../crates/recite-ui/resources/inventory.toml");
const inventorySource = readFileSync(inventoryPath, "utf8");
const vscodeProjection = parseProjectionInventory(inventorySource, "vscode");
const vscodiumProjection = parseProjectionInventory(inventorySource, "vscodium");
assertProjectionParity(vscodeProjection, vscodiumProjection);

export const RUNTIME_MESSAGE_IDS = Object.freeze(vscodeProjection.runtimeIds);
export const PACKAGE_MESSAGE_IDS = Object.freeze(vscodeProjection.packageIds);

// These are the extension-owned visible messages. Every source use must go
// through clientMessage and resolve to one of these projected Fluent IDs.
export const SOURCE_MESSAGE_IDS = Object.freeze([...RUNTIME_MESSAGE_IDS]);

export function projectMessages(fluent) {
  const canonical = parseRepresentableMessages(fluent, [
    ...RUNTIME_MESSAGE_IDS,
    ...PACKAGE_MESSAGE_IDS
  ]);
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

/**
 * Parse the inventory-owned subset of Fluent. Host projections deliberately
 * support only single-line templates with simple variable placeables. A
 * continuation, selector, term, attribute, or other Fluent expression must
 * fail explicitly instead of being silently truncated by a line regex.
 */
export function parseRepresentableMessages(source, ids) {
  const wanted = new Set(ids);
  const messages = new Map();
  let currentId;
  for (const [index, line] of source.split(/\r?\n/u).entries()) {
    const message = /^(?<id>[a-z0-9-]+) = (?<value>[^\r\n]*)$/u.exec(line);
    if (message) {
      currentId = message.groups.id;
      if (!wanted.has(currentId)) continue;
      if (messages.has(currentId)) {
        throw new Error(`duplicate canonical Fluent message ${currentId} at line ${index + 1}`);
      }
      const value = message.groups.value;
      assertRepresentableValue(currentId, value, index + 1);
      messages.set(currentId, value);
      continue;
    }
    if (/^\s/u.test(line) && currentId && wanted.has(currentId)) {
      throw new Error(
        `canonical Fluent message ${currentId} uses a continuation at line ${index + 1}; ` +
        "VS Code and Neovim projections support only single-line templates"
      );
    }
    if (line.trim() !== "") currentId = undefined;
  }
  for (const id of wanted) {
    if (!messages.has(id)) throw new Error(`canonical Fluent message is missing ${id}`);
  }
  return messages;
}

function assertRepresentableValue(id, value, line) {
  if (!/^([^{}]|\{\$[a-zA-Z][a-zA-Z0-9_-]*\})*$/u.test(value)) {
    throw new Error(`canonical Fluent message ${id} uses an unsupported expression at line ${line}`);
  }
}

function parseProjectionInventory(source, name) {
  const marker = `[projections.${name}]`;
  const start = source.indexOf(marker);
  if (start < 0) throw new Error(`canonical inventory is missing ${marker}`);
  const section = source.slice(start + marker.length).split(/\n\[/u, 1)[0];
  const sourceResource = scalar(section, "source_resource");
  const runtimeOutput = scalar(section, "runtime_output");
  const packageOutput = scalar(section, "package_output");
  const runtimeIds = array(section, "runtime_ids");
  const packageIds = array(section, "package_ids");
  if (!sourceResource || !runtimeOutput || !packageOutput || !runtimeIds.length || !packageIds.length) {
    throw new Error(`${marker} must declare source, outputs, and IDs`);
  }
  return { sourceResource, runtimeOutput, packageOutput, runtimeIds, packageIds };
}

function scalar(section, key) {
  return section.match(new RegExp(`(?:^|\\n)${key}\\s*=\\s*"([^"\\n]+)"`, "u"))?.[1];
}

function array(section, key) {
  const match = section.match(new RegExp(`(?:^|\\n)${key}\\s*=\\s*\\[([\\s\\S]*?)\\]`, "u"));
  if (!match) return [];
  const values = [...match[1].matchAll(/"([a-z0-9-]+)"/gu)].map((entry) => entry[1]);
  if (new Set(values).size !== values.length) throw new Error(`${key} contains duplicate IDs`);
  return values;
}

function assertProjectionParity(left, right) {
  for (const key of ["sourceResource", "runtimeOutput", "packageOutput"]) {
    if (left[key] !== right[key]) throw new Error(`VS Code/VSCodium projection ${key} diverges`);
  }
  for (const key of ["runtimeIds", "packageIds"]) {
    if (JSON.stringify(left[key]) !== JSON.stringify(right[key])) {
      throw new Error(`VS Code/VSCodium projection ${key} diverges`);
    }
  }
}

export function renderMessageProjections(projections) {
  const runtimeOutput = path.relative(packageRoot, path.resolve(packageRoot, "../../", vscodeProjection.runtimeOutput));
  const packageOutput = path.relative(packageRoot, path.resolve(packageRoot, "../../", vscodeProjection.packageOutput));
  return {
    [runtimeOutput]:
      `// Generated from crates/recite-ui/resources/en-US.ftl. Do not edit.\nexport default Object.freeze(${JSON.stringify(projections.runtime, null, 2)});\n`,
    [packageOutput]: `${JSON.stringify(projections.package, null, 2)}\n`
  };
}

export async function verifyMessageProjections(packageRoot) {
  const fluent = await readFile(
    path.resolve(packageRoot, "../../crates/recite-ui/resources", vscodeProjection.sourceResource), "utf8"
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
    path.resolve(packageRoot, "../../crates/recite-ui/resources", vscodeProjection.sourceResource), "utf8"
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
    destination.originallyExisted = await assertSafeDestination(
      packageRoot, destination.relative, fileSystem
    );
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
      if (destination.originallyExisted) {
        await assertSafeAbsent(destination.backupPath, fileSystem);
        await fileSystem.copyFile(destination.target, destination.backupPath);
      }
    }

    for (const destination of destinations) {
      const stillExists = await assertSafeDestination(packageRoot, destination.relative, fileSystem);
      if (stillExists !== destination.originallyExisted) {
        throw new Error(`projection changed during update: ${destination.relative}`);
      }
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
      if (index === components.length - 1 && error?.code === "ENOENT") return false;
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
  return true;
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
      if (destination.originallyExisted) {
        await fileSystem.rename(destination.backupPath, destination.target);
      }
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
