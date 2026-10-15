import { is, traceSchema } from "./schemas/index.js";
import type { Trace } from "./schemas/index.js";

export function validTrace(value: unknown): value is Trace {
  return is(traceSchema, value);
}
