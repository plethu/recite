import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";
import {
  parseDiagnosticMessages,
  parseRepresentableMessages,
} from "../../message-projection-parser.mjs";
import {
  projectDiagnostics,
  renderDiagnostics,
  verifyDiagnosticProjection,
} from "../scripts/diagnostic-projections.mjs";

const root = path.resolve(import.meta.dirname, "..");

test("diagnostic projection preserves named presentation arguments", async () => {
  const source = await readFile(
    path.resolve(root, "../../crates/recite-ui/resources/diagnostics.ftl"),
    "utf8",
  );
  const inventory = await readFile(
    path.resolve(root, "../../crates/recite-ui/resources/inventory.toml"),
    "utf8",
  );
  const values = projectDiagnostics(source, inventory);
  assert.equal(values["diagnostic-parse-013"], undefined);
  assert.deepEqual(values["diagnostic-validate-007"], {
    template: "unknown block reference `{$reference}`",
    arguments: [{ name: "reference", type: "string" }],
  });
  assert.deepEqual(values["diagnostic-parse-012-unexpected-character"], {
    template: "malformed effect statement: unexpected character '{$character}'",
    arguments: [{ name: "character", type: "string" }],
  });
  assert.deepEqual(values["diagnostic-parse-034-expected-directive"], {
    template: "expected PO directive",
    arguments: [],
  });
  assert.deepEqual(values["diagnostic-parse-013-nesting-limit"], {
    template:
      "malformed condition expression: condition syntax nesting exceeds the limit of {$limit}",
    arguments: [{ name: "limit", type: "integer" }],
  });
  await assert.doesNotReject(verifyDiagnosticProjection());
});

test("Fluent AST projection lowers whitespace and repeated variables to host tokens", () => {
  const source = "# Canonical message\nfixture = limit { $limit }, again {$limit}, {  $detail  }\n";
  const expected = new Map([["fixture", "limit {$limit}, again {$limit}, {$detail}"]]);
  assert.deepEqual(parseRepresentableMessages(source, ["fixture"]), expected);
  assert.deepEqual(
    parseRepresentableMessages(source.replaceAll("\n", "\r\n"), ["fixture"]),
    expected,
  );
  assert.throws(() => parseRepresentableMessages(source, ["missing"]), /is missing missing/);
});

test("canonical Fluent syntax and duplicate failures cannot become diagnostic fallback", () => {
  const fixtures = [
    ["diagnostic-fixture =\n", /malformed canonical Fluent/],
    ["diagnostic-fixture = broken { $limit\n", /malformed canonical Fluent/],
    ["unselected = broken { $limit\ndiagnostic-fixture = text\n", /malformed canonical Fluent/],
    ["diagnostic-fixture = { $limit -> [one] one *[other] other }\n", /malformed canonical Fluent/],
    [
      "diagnostic-fixture = first\ndiagnostic-fixture = second\n",
      /duplicate canonical Fluent message/,
    ],
    [
      "unselected = first\nunselected = second\ndiagnostic-fixture = text\n",
      /duplicate canonical Fluent message/,
    ],
  ];
  for (const [source, failure] of fixtures) {
    assert.throws(() => parseRepresentableMessages(source, ["diagnostic-fixture"]), failure);
    assert.throws(() => parseDiagnosticMessages(source), failure);
  }
});

test("host projections reject attributes and plain multiline patterns", () => {
  const fixtures = [
    ["diagnostic-fixture = text\n    .hint = detail\n", /unsupported attribute/],
    ["diagnostic-fixture =\n    .hint = detail\n", /unsupported attribute/],
    ["diagnostic-fixture = first\n    second\n", /continuation/],
    ["diagnostic-fixture =\n    block value\n", /continuation/],
    ["diagnostic-fixture = {\n    $limit\n}\n", /continuation/],
  ];
  for (const [source, failure] of fixtures) {
    assert.throws(() => parseRepresentableMessages(source, ["diagnostic-fixture"]), failure);
    assert.throws(() => parseDiagnosticMessages(source), failure);
  }
});

test("valid unsupported Fluent expressions fall back only for diagnostic projections", () => {
  const expressions = [
    "{ $limit ->\n    [one] one\n   *[other] other\n}",
    "{-term}",
    "{other-message}",
    "{NUMBER($limit)}",
    "{1}",
    "{\"literal\"}",
    "{{ $limit }}",
  ];
  for (const expression of expressions) {
    const source = `diagnostic-fixture = ${expression}\ndiagnostic-fixture-help = explain\n`;
    assert.throws(
      () => parseRepresentableMessages(source, ["diagnostic-fixture"]),
      /unsupported expression/,
    );
    assert.deepEqual(parseDiagnosticMessages(source), new Map());
  }
});

test("diagnostic projection verification detects stale output", async () => {
  const source = await readFile(
    path.resolve(root, "../../crates/recite-ui/resources/diagnostics.ftl"),
    "utf8",
  );
  const inventory = await readFile(
    path.resolve(root, "../../crates/recite-ui/resources/inventory.toml"),
    "utf8",
  );
  const expected = renderDiagnostics(projectDiagnostics(source, inventory));
  assert.match(expected, /diagnostic-parse-001/);
});
