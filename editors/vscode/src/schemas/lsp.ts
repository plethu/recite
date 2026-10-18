import * as v from "valibot";
const coordinate = v.pipe(v.number(), v.integer(), v.minValue(0));
const position = v.strictObject({ line: coordinate, character: coordinate });
export const lspRangeSchema = v.pipe(
  v.strictObject({ start: position, end: position }),
  v.check(
    ({ start, end }) =>
      start.line < end.line || (start.line === end.line && start.character <= end.character),
  ),
);
export const prepareRenameSchema = v.union([
  lspRangeSchema,
  v.strictObject({ range: lspRangeSchema, placeholder: v.string() }),
  v.strictObject({ defaultBehavior: v.literal(true) }),
]);
// LSP objects permit extensions, but resource operations and unversioned edits
// do not have authority to modify open documents through this adapter.
export const workspaceEditSchema = v.object({
  documentChanges: v.array(
    v.object({
      textDocument: v.object({
        uri: v.pipe(v.string(), v.nonEmpty()),
        version: v.pipe(v.number(), v.integer()),
      }),
      edits: v.array(v.object({ range: lspRangeSchema, newText: v.string() })),
    }),
  ),
});
export const renameEditSchema = v.pipe(
  v.looseObject({ documentChanges: v.array(v.unknown()) }),
  v.check((value) => value.documentChanges.length > 0 && !Object.hasOwn(value, "changes")),
);
export const commandPathSchema = v.object({ fsPath: v.string() });
export const codeActionInputSchema = v.object({
  title: v.optional(v.string()),
  kind: v.optional(v.string()),
  command: v.optional(
    v.union([
      v.string(),
      v.object({
        title: v.string(),
        command: v.string(),
        arguments: v.optional(v.array(v.unknown())),
      }),
    ]),
  ),
  disabled: v.optional(v.object({ reason: v.optional(v.string()) })),
  edit: v.optional(v.unknown()),
  isPreferred: v.optional(v.boolean()),
  diagnostics: v.optional(v.array(v.unknown())),
});
export const lspDiagnosticSchema = v.object({
  range: lspRangeSchema,
  message: v.string(),
  severity: v.optional(v.number()),
  code: v.optional(v.union([v.string(), v.number()])),
  source: v.optional(v.string()),
});

export const stringArraySchema = v.array(v.string());
