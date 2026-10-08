import assert from "node:assert/strict";
import test from "node:test";
import { validStructuredError } from "../src/watch-record-validation.js";

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
  for (
    const operation of ["control", "dispatch", "export_schema", "render", "not-a-wire-operation"]
  ) {
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
