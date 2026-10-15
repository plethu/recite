export const StartupOutcomeKind = Object.freeze({
  Started: "started",
  RetryableFailure: "retryable-failure",
  Refused: "refused",
});

export type StartupOutcome = Readonly<{
  kind: (typeof StartupOutcomeKind)[keyof typeof StartupOutcomeKind];
  error: unknown;
  reported: boolean;
}>;

export function startupOutcome(
  kind: StartupOutcome["kind"],
  error: unknown = undefined,
  reported = false,
): StartupOutcome {
  return Object.freeze({ kind, error, reported });
}
