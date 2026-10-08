import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";
import { projectDiagnostics } from "../scripts/diagnostic-projections.mjs";

test("Neovim diagnostic projection validates shared contracts with CRLF input", () => {
  const source = "diagnostic-hostile = bad {$actual}\r\n";
  const contract = "# generated\r\ndiagnostic-hostile\tactual\tstring\r\n";

  assert.deepEqual(
    projectDiagnostics(source, contract).values.get("diagnostic-hostile"),
    { template: "bad {$actual}", arguments: [{ name: "actual", type: "string" }] },
  );
});

test("Neovim diagnostic projection lowers the canonical spaced nesting limit variable", async () => {
  const resources = path.resolve(import.meta.dirname, "../../../crates/recite-ui/resources");
  const [source, contracts] = await Promise.all([
    readFile(path.join(resources, "diagnostics.ftl"), "utf8"),
    readFile(path.join(resources, "vscode-diagnostic-contract.tsv"), "utf8"),
  ]);
  assert.deepEqual(
    projectDiagnostics(source, contracts).values.get("diagnostic-parse-013-nesting-limit"),
    {
      template:
        "malformed condition expression: condition syntax nesting exceeds the limit of {$limit}",
      arguments: [{ name: "limit", type: "integer" }],
    },
  );
});

test("Neovim keeps contracts when valid expressions require diagnostic fallback", () => {
  const source = "diagnostic-fixture = { $count ->\n    [one] one\n   *[other] other\n}\n";
  const contract = "diagnostic-fixture\tcount\tinteger\n";
  const projection = projectDiagnostics(source, contract);
  assert.equal(projection.values.has("diagnostic-fixture"), false);
  assert.deepEqual(projection.contracts.get("diagnostic-fixture"), [{
    name: "count",
    type: "integer",
  }]);
  assert.throws(
    () => projectDiagnostics("diagnostic-fixture = { $count\n", contract),
    /malformed canonical Fluent/,
  );
});

test("Neovim diagnostic projection rejects a contract placeholder mismatch", () => {
  const source = "diagnostic-hostile = bad {$actual}\n";
  const contract = "diagnostic-hostile\texpected\tstring\n";

  assert.throws(
    () => projectDiagnostics(source, contract),
    /diagnostic contract\/template mismatch for diagnostic-hostile/,
  );
});
