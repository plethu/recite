import { lstatSync, readdirSync } from "node:fs";
import path from "node:path";

export function assertSafeTree(directory, label = "extension tree") {
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const full = path.join(directory, entry.name);
    const stat = lstatSync(full);
    if (stat.isSymbolicLink()) throw new Error(`refusing symlink in ${label}: ${full}`);
    if (stat.isDirectory()) assertSafeTree(full, label);
    else if (!stat.isFile()) throw new Error(`refusing non-regular ${label} entry: ${full}`);
  }
}

export function assertRegularFile(file, label = "extension package entry") {
  const stat = lstatSync(file);
  if (!stat.isFile()) throw new Error(`refusing non-regular ${label}: ${file}`);
}
