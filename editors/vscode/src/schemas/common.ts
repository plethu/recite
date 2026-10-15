import * as v from "valibot";
import { integerInRange, protocol, type IntegerValue } from "../command-protocol.js";

export const nonemptyString = v.pipe(v.string(), v.minLength(1));
export const positiveInteger = v.pipe(v.number(), v.safeInteger(), v.minValue(1));
export const unsignedInteger = v.custom<IntegerValue>((value) =>
  integerInRange(value, "0", "18446744073709551615"),
);
export const signedInteger = v.custom<IntegerValue>((value) =>
  integerInRange(value, "-9223372036854775808", "9223372036854775807"),
);
export const wideUnsignedInteger = v.custom<IntegerValue>((value) =>
  integerInRange(value, "0", "340282366920938463463374607431768211455"),
);
export const machinePathSchema = v.variant("encoding", [
  v.strictObject({ encoding: v.literal("utf8"), value: v.string() }),
  v.strictObject({
    encoding: v.literal("unix_bytes"),
    value: v.pipe(v.string(), v.regex(/^(?:[0-9a-f]{2})*$/u)),
  }),
  v.strictObject({
    encoding: v.literal("windows_wtf16"),
    value: v.array(v.pipe(v.number(), v.safeInteger(), v.minValue(0), v.maxValue(65535))),
  }),
]);
export const artifactSchema = v.strictObject({
  path: machinePathSchema,
  size_bytes: unsignedInteger,
});
export const scalarPositionSchema = v.strictObject({
  line: positiveInteger,
  column: positiveInteger,
});
export const sortedStrings = v.pipe(
  v.array(v.string()),
  v.check((values) =>
    values.every((value, index) => index === 0 || compareUtf8(values[index - 1]!, value) < 0),
  ),
);
export const uniqueStrings = v.pipe(
  v.array(v.string()),
  v.check((values) => new Set(values).size === values.length),
);
export function compareUtf8(left: string, right: string) {
  return Buffer.compare(Buffer.from(left), Buffer.from(right));
}

// Wire schemas validate without coercions, defaults, or transforms, retaining
// the original record and LosslessInteger instances after the type guard.
export function parseWire<T extends v.GenericSchema>(
  schema: T,
  input: unknown,
  code: string,
): v.InferInput<T> {
  if (!v.is(schema, input)) throw protocol(code);
  return input;
}
