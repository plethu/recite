import { cp, mkdir, rm } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { assertSafeTree } from "./safety.mjs";

const root = path.dirname(fileURLToPath(import.meta.url));
const packageRoot = path.resolve(root, "..");
const sourceRoot = path.join(packageRoot, "src");
const outputRoot = path.join(packageRoot, "dist");

assertSafeTree(sourceRoot, "extension source");
await rm(outputRoot, { recursive: true, force: true });
await mkdir(outputRoot, { recursive: true });
await cp(sourceRoot, outputRoot, { recursive: true });
