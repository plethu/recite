export const StartupOutcomeKind = Object.freeze({
  Started: "started",
  RetryableFailure: "retryable-failure",
  Refused: "refused"
});

export function startupOutcome(kind, error, reported = false) {
  return Object.freeze({ kind, error, reported });
}
