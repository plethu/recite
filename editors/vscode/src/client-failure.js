export const ClientFailureKind = Object.freeze({
  Transport: "transport",
  Protocol: "protocol",
  Lifecycle: "lifecycle"
});

/**
 * A failure category owned by the extension boundary. `detail` is reserved
 * for an operating-system, child-process, or server-provided payload; the
 * category itself is always rendered through the shared UI catalog.
 */
export class ClientFailure extends Error {
  constructor(kind, detail, cause) {
    super(detail ?? kind, cause === undefined ? undefined : { cause });
    this.name = "ReciteClientFailure";
    this.kind = kind;
    this.detail = detail;
    this.code = cause?.code;
  }
}

export function asClientFailure(kind, error) {
  if (error instanceof ClientFailure) return error;
  const detail = error && typeof error.message === "string"
    ? error.message
    : String(error);
  return new ClientFailure(kind, detail, error);
}

export function isClientFailure(value) {
  return value instanceof ClientFailure &&
    Object.values(ClientFailureKind).includes(value.kind);
}
