import test from "node:test";
import assert from "node:assert/strict";
import { projectDiagnostics } from "../scripts/diagnostic-projections.mjs";

test("Neovim diagnostic projection validates shared contracts with CRLF input", () => {
  const source = "diagnostic-hostile = bad {$actual}\r\n";
  const contract = "# generated\r\ndiagnostic-hostile\tactual\tstring\r\n";

  assert.deepEqual(
    projectDiagnostics(source, contract).values.get("diagnostic-hostile"),
    { template: "bad {$actual}", arguments: [{ name: "actual", type: "string" }] }
  );
});

test("Neovim diagnostic projection rejects a contract placeholder mismatch", () => {
  const source = "diagnostic-hostile = bad {$actual}\n";
  const contract = "diagnostic-hostile\texpected\tstring\n";

  assert.throws(
    () => projectDiagnostics(source, contract),
    /diagnostic contract\/template mismatch for diagnostic-hostile/
  );
});
