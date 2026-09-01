import { lstatSync, readdirSync } from "node:fs";
import path from "node:path";

export const SOURCE_MODULE_EXTENSIONS = Object.freeze([".js", ".mjs", ".cjs"]);

export function listSourceModules(sourceRoot) {
  const root = path.resolve(sourceRoot);
  assertDirectory(root, "extension source root");
  return walk(root, root).sort((left, right) => left.relativePath.localeCompare(right.relativePath));
}

function walk(root, directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const absolutePath = path.join(directory, entry.name);
    const stat = lstatSync(absolutePath);
    if (stat.isSymbolicLink()) throw new Error(`refusing symlink in extension source: ${absolutePath}`);
    if (stat.isDirectory()) return walk(root, absolutePath);
    if (!stat.isFile() || !SOURCE_MODULE_EXTENSIONS.some((extension) => entry.name.endsWith(extension))) {
      return [];
    }

    const relativePath = path.relative(root, absolutePath).split(path.sep).join("/");
    if (!relativePath || relativePath.startsWith("../") || path.isAbsolute(relativePath) ||
        path.posix.normalize(relativePath) !== relativePath) {
      throw new Error(`refusing unsafe extension source path: ${relativePath}`);
    }
    return [{ relativePath, absolutePath }];
  });
}

function assertDirectory(directory, label) {
  const stat = lstatSync(directory);
  if (stat.isSymbolicLink()) throw new Error(`refusing symlink in ${label}: ${directory}`);
  if (!stat.isDirectory()) throw new Error(`refusing non-directory ${label}: ${directory}`);
}
