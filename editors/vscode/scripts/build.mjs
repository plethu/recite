import { cp, rm, readFile, readdir, writeFile } from "node:fs/promises";
import { build } from "tsdown";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { verifyDiagnosticProjection } from "./diagnostic-projections.mjs";
import { verifyMessageProjections } from "./message-projections.mjs";
import { assertSafeTree } from "./safety.mjs";

const root = path.dirname(fileURLToPath(import.meta.url));
const packageRoot = path.resolve(root, "..");
const sourceRoot = path.join(packageRoot, "src");
const outputRoot = path.join(packageRoot, "dist");

await verifyMessageProjections(packageRoot);
await verifyDiagnosticProjection();
assertSafeTree(sourceRoot, "extension source");
await build({
  config: false,
  entry: (await readdir(sourceRoot, { withFileTypes: true }))
    .filter((entry) => entry.isFile() && entry.name.endsWith(".ts"))
    .map((entry) => path.join(sourceRoot, entry.name)),
  outDir: outputRoot,
  platform: "node",
  target: "es2023",
  format: "esm",
  outExtensions: () => ({ js: ".js" }),
  unbundle: true,
  dts: false,
  sourcemap: false,
  deps: {
    neverBundle: ["vscode", "vscode-languageclient/node.js", "valibot", /schemas\/index\.js$/],
    onlyBundle: [],
  },
});
await build({
  config: false,
  entry: [path.join(sourceRoot, "extension.cts")],
  outDir: outputRoot,
  platform: "node",
  target: "es2023",
  format: "cjs",
  clean: false,
  dts: false,
  sourcemap: false,
  deps: { neverBundle: ["vscode", "./extension.js"] },
});
// Bundle only the schema entry: imports keep their original lossless integer
// class and generated vocabulary, while unused Valibot code is eliminated.
await rm(path.join(outputRoot, "schemas"), { recursive: true, force: true });
await build({
  config: false,
  entry: { index: path.join(sourceRoot, "schemas/index.ts") },
  outDir: path.join(outputRoot, "schemas"),
  platform: "node",
  target: "es2023",
  format: "esm",
  clean: false,
  dts: false,
  sourcemap: false,
  minify: true,
  outExtensions: () => ({ js: ".js" }),
  deps: {
    onlyBundle: ["valibot"],
    neverBundle: [/command-protocol\.(?:js|ts)$/, /\.generated\.js$/],
  },
});
const validatorEntry = fileURLToPath(import.meta.resolve("valibot"));
await writeFile(
  path.join(outputRoot, "valibot.LICENSE"),
  await readFile(path.resolve(path.dirname(validatorEntry), "../LICENSE.md")),
);
for (const file of ["messages", "diagnostics", "diagnostic-contract", "error-vocabulary"]) {
  await cp(
    path.join(sourceRoot, `${file}.generated.js`),
    path.join(outputRoot, `${file}.generated.js`),
  );
}
