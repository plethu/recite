const DEFAULT_RESTART_DELAYS_MS = [100, 500, 1_000, 2_000, 5_000];

/** Owns bounded restart budgeting separately from controller orchestration. */
export class RestartPolicy {
  declare delays: number[];
  declare attempt: number;
  declare exhaustedReported: boolean;

  constructor(delays: readonly number[] = DEFAULT_RESTART_DELAYS_MS) {
    this.delays = [...delays];
    this.attempt = 0;
    this.exhaustedReported = false;
  }

  nextDelay() {
    if (this.attempt >= this.delays.length) return undefined;
    return this.delays[this.attempt++];
  }

  reportExhausted() {
    if (this.exhaustedReported) return false;
    this.exhaustedReported = true;
    return true;
  }

  reset() {
    this.attempt = 0;
    this.exhaustedReported = false;
  }
}
