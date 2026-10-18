import * as v from "valibot";
import { artifactSchema, nonemptyString, positiveInteger, unsignedInteger } from "./common.ts";
import { diagnosticRecordSchema } from "./diagnostic.ts";
import { structuredErrorSchema } from "./watch.ts";
import { traceSchema } from "./trace.ts";
const envelope = {
  version: v.literal(1),
  sequence: unsignedInteger,
  command: v.string(),
  invocation_id: v.optional(v.string()),
};
export const startedSchema = v.strictObject({ ...envelope, event: v.literal("command.started") });
export const resultSchema = v.pipe(
  v.strictObject({
    ...envelope,
    event: v.literal("command.result"),
    status: v.picklist(["success", "content_diagnostics"]),
    exit_code: v.picklist([0, 1]),
    data: v.unknown(),
  }),
  v.check((value) => value.exit_code === (value.status === "success" ? 0 : 1)),
);
export const errorSchema = v.strictObject({
  ...envelope,
  event: v.literal("command.error"),
  status: v.literal("failure"),
  exit_code: v.literal(1),
  error: structuredErrorSchema,
});
export const diagnosticsDataSchema = v.strictObject({
  diagnostics: v.array(diagnosticRecordSchema),
});
export const artifactDataSchema = v.strictObject({
  diagnostics: v.array(diagnosticRecordSchema),
  artifact: artifactSchema,
});
const catalogEntrySchema = v.strictObject({
  context: nonemptyString,
  source_text: nonemptyString,
  plural_source_text: v.nullable(v.string()),
  comments: v.array(v.string()),
  reference: v.nullable(
    v.strictObject({ file: nonemptyString, line: positiveInteger, column: positiveInteger }),
  ),
});
export const extractDataSchema = v.union([
  artifactDataSchema,
  v.strictObject({
    diagnostics: v.array(diagnosticRecordSchema),
    entries: v.array(catalogEntrySchema),
  }),
]);
export const runtimeDataSchema = v.strictObject({ trace: traceSchema });
export type FiniteData = v.InferInput<
  | typeof diagnosticsDataSchema
  | typeof artifactDataSchema
  | typeof extractDataSchema
  | typeof runtimeDataSchema
>;
export type FiniteTerminal =
  | (v.InferInput<typeof resultSchema> & { data: FiniteData })
  | v.InferInput<typeof errorSchema>;
