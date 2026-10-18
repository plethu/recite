import * as v from "valibot";
import diagnosticContracts from "../diagnostic-contract.generated.js";
import { scalarPositionSchema, signedInteger, nonemptyString } from "./common.ts";

export const diagnosticArgumentSchema = v.variant("type", [
  v.strictObject({ type: v.literal("string"), value: v.string() }),
  v.strictObject({ type: v.literal("integer"), value: signedInteger }),
  v.strictObject({ type: v.literal("float"), value: v.pipe(v.number(), v.finite()) }),
  v.strictObject({ type: v.literal("boolean"), value: v.boolean() }),
]);
const definitions = new Map(Object.entries(diagnosticContracts));
export const presentationSchema = v.pipe(
  v.strictObject({
    id: v.pipe(v.string(), v.regex(/^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$/u)),
    arguments: v.record(
      v.pipe(v.string(), v.regex(/^[a-z][a-z0-9_]*$/u)),
      diagnosticArgumentSchema,
    ),
  }),
  v.check((presentation) => {
    const definition = definitions.get(presentation.id);
    if (!definition) return true;
    return (
      Object.keys(presentation.arguments).length === definition.arguments.length &&
      definition.arguments.every(({ name, type }) => presentation.arguments[name]?.type === type)
    );
  }),
);
export const diagnosticSpanSchema = v.pipe(
  v.strictObject({
    file: nonemptyString,
    start: scalarPositionSchema,
    end: v.nullable(scalarPositionSchema),
  }),
  v.check(
    ({ start, end }) =>
      end === null ||
      end.line > start.line ||
      (end.line === start.line && end.column >= start.column),
  ),
);
const explanationSchema = v.strictObject({
  meaning: presentationSchema,
  common_causes: v.array(presentationSchema),
  remediation: v.array(presentationSchema),
});
export const diagnosticRecordSchema = v.pipe(
  v.strictObject({
    version: v.literal(1),
    code: v.pipe(v.string(), v.regex(/^[A-Z][A-Z0-9]*_[A-Z0-9]+$/u)),
    severity: v.picklist(["error", "warning", "information", "hint"]),
    span: diagnosticSpanSchema,
    presentation: presentationSchema,
    related: v.array(
      v.strictObject({ span: diagnosticSpanSchema, presentation: presentationSchema }),
    ),
    help: v.undefinedable(v.nullable(presentationSchema)),
    explanation: v.undefinedable(v.nullable(explanationSchema)),
    compatibility_message: v.undefinedable(v.nullable(v.string())),
  }),
  v.check((value) =>
    ["help", "explanation", "compatibility_message"].every((key) => Object.hasOwn(value, key)),
  ),
);
export type DiagnosticRecord = v.InferInput<typeof diagnosticRecordSchema>;
export type DiagnosticSpan = v.InferInput<typeof diagnosticSpanSchema>;
export type Presentation = v.InferInput<typeof presentationSchema>;
export type DiagnosticArgument = v.InferInput<typeof diagnosticArgumentSchema>;
export type ScalarPosition = v.InferInput<typeof scalarPositionSchema>;
