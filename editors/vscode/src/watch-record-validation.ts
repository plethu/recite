import { is, structuredErrorSchema } from "./schemas/index.js";
import type { StructuredError } from "./schemas/index.js";

export function validStructuredError(value: unknown): value is StructuredError {
  return is(structuredErrorSchema, value);
}
