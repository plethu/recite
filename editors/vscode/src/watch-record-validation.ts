import {
  is,
  buildStartSchema,
  buildCompletedSchema,
  watchStopSchema,
  cancellationSchema,
  structuredErrorSchema,
} from "./schemas/index.js";
import type {
  BuildStart,
  BuildCompleted,
  WatchStop,
  Cancellation,
  StructuredError,
} from "./schemas/index.js";

export function validBuildStart(value: unknown): value is BuildStart {
  return is(buildStartSchema, value);
}
export function validBuildCompleted(value: unknown): value is BuildCompleted {
  return is(buildCompletedSchema, value);
}
export function validStop(value: unknown): value is WatchStop {
  return is(watchStopSchema, value);
}
export function validCancellation(value: unknown): value is Cancellation {
  return is(cancellationSchema, value);
}
export function validStructuredError(value: unknown): value is StructuredError {
  return is(structuredErrorSchema, value);
}
