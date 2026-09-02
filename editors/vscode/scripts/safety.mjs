import { lstatSync, readdirSync } from "node:fs";
import path from "node:path";

export function assertSafeTree(directory, label = "extension tree") {
  const root = path.resolve(directory);
  const rootStat = lstatSync(root);
  if (rootStat.isSymbolicLink()) throw new Error(`refusing symlink in ${label}: ${root}`);
  if (!rootStat.isDirectory()) throw new Error(`refusing non-directory ${label}: ${root}`);
  assertSafeDirectory(root, label);
}

function assertSafeDirectory(directory, label) {
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const full = path.join(directory, entry.name);
    const stat = lstatSync(full);
    if (stat.isSymbolicLink()) throw new Error(`refusing symlink in ${label}: ${full}`);
    if (stat.isDirectory()) assertSafeDirectory(full, label);
    else if (!stat.isFile()) throw new Error(`refusing non-regular ${label} entry: ${full}`);
  }
}

export function assertRegularFile(file, label = "extension package entry") {
  const stat = lstatSync(file);
  if (!stat.isFile()) throw new Error(`refusing non-regular ${label}: ${file}`);
}

export function assertContainedRegularFile(root, relative, label = "package entry") {
  const base = path.resolve(root);
  const baseStat = lstatSync(base);
  if (baseStat.isSymbolicLink()) throw new Error(`refusing symlink in ${label}: ${base}`);
  if (!baseStat.isDirectory()) throw new Error(`refusing non-directory ${label} root: ${base}`);
  const target = path.resolve(base, relative);
  const remainder = path.relative(base, target);
  if (!remainder || remainder === ".." || remainder.startsWith(`..${path.sep}`) || path.isAbsolute(remainder)) {
    throw new Error(`refusing path outside ${label} root: ${relative}`);
  }

  let current = base;
  const components = remainder.split(path.sep);
  for (const [index, component] of components.entries()) {
    current = path.join(current, component);
    const stat = lstatSync(current);
    if (stat.isSymbolicLink()) throw new Error(`refusing symlink in ${label}: ${current}`);
    if (index < components.length - 1 && !stat.isDirectory()) {
      throw new Error(`refusing non-directory component in ${label}: ${current}`);
    }
  }
  assertRegularFile(target, label);
  return target;
}
