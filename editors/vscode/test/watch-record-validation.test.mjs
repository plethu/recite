import assert from "node:assert/strict";
import { test } from "vitest";
import { validStructuredError } from "../src/watch-record-validation.ts";

import { errorCategories, errorCodes, operations } from "../src/error-vocabulary.generated.js";

test("structured errors accept every producer operation and reject internal paths", () => {
  for (const operation of operations) {
    assert.equal(
      validStructuredError({
        category: "input",
        code: "io",
        operation,
      }),
      true,
      `producer operation should be accepted: ${operation}`,
    );
  }
  for (const operation of [
    "control",
    "dispatch",
    "export_schema",
    "render",
    "not-a-wire-operation",
  ]) {
    assert.equal(
      validStructuredError({
        category: "input",
        code: "io",
        operation,
      }),
      false,
      `non-wire operation should be rejected: ${operation}`,
    );
  }
});

test("structured errors accept generated categories and codes without accepting transport tags", () => {
  for (const category of errorCategories) {
    assert.equal(validStructuredError({ category, code: "io", operation: "validate" }), true);
  }
  for (const code of errorCodes) {
    assert.equal(validStructuredError({ category: "input", code, operation: "validate" }), true);
  }
  for (const code of ["import", "import_json", "structured_stderr", "not-a-wire-code"]) {
    assert.equal(validStructuredError({ category: "input", code, operation: "validate" }), false);
  }
});

test("recovery I/O details accept producer fields and reject unsafe integers and extras", async () => {
  const { is, recoverySchema } = await import("../src/schemas/index.ts");
  const recovery = {
    marker: { encoding: "utf8", value: "/project/.recite/recovery" },
    reason: "stage_cleanup_failed",
    detail: { type: "io", kind: "permission_denied", raw_os_error: 13 },
  };
  assert.equal(is(recoverySchema, recovery), true);
  assert.equal(
    is(recoverySchema, { ...recovery, detail: { ...recovery.detail, raw_os_error: null } }),
    true,
  );
  assert.equal(
    is(recoverySchema, { ...recovery, detail: { ...recovery.detail, raw_os_error: 2 ** 53 } }),
    false,
  );
  assert.equal(
    is(recoverySchema, { ...recovery, detail: { ...recovery.detail, unexpected: true } }),
    false,
  );
});
