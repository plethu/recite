import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { projectDiagnostics, renderDiagnostics, verifyDiagnosticProjection } from "../scripts/diagnostic-projections.mjs";

const root = path.resolve(import.meta.dirname, "..");

test("diagnostic projection preserves named presentation arguments", async () => {
  const source = await readFile(path.resolve(root, "../../crates/recite-ui/resources/diagnostics.ftl"), "utf8");
  const inventory = await readFile(path.resolve(root, "../../crates/recite-ui/resources/inventory.toml"), "utf8");
  const values = projectDiagnostics(source, inventory);
  assert.deepEqual(values["diagnostic-validate-007"], {
    template: "unknown block reference `{$reference}`",
    arguments: [{ name: "reference", type: "string" }]
  });
  assert.deepEqual(values["diagnostic-parse-012-unexpected-character"], {
    template: "malformed effect statement: unexpected character '{$character}'",
    arguments: [{ name: "character", type: "string" }]
  });
  assert.deepEqual(values["diagnostic-parse-034-expected-directive"], {
    template: "expected PO directive", arguments: []
  });
  await assert.doesNotReject(verifyDiagnosticProjection());
});

test("diagnostic projection verification detects stale output", async () => {
  const source = await readFile(path.resolve(root, "../../crates/recite-ui/resources/diagnostics.ftl"), "utf8");
  const inventory = await readFile(path.resolve(root, "../../crates/recite-ui/resources/inventory.toml"), "utf8");
  const expected = renderDiagnostics(projectDiagnostics(source, inventory));
  assert.match(expected, /diagnostic-parse-001/);
});
