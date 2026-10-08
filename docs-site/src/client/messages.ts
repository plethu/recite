export const messageNames = [
  "ready",
  "loading",
  "loadingCompiler",
  "downloadTimeout",
  "resourceLimit",
  "timeout",
  "failed",
  "tooLarge",
  "changed",
  "stopped",
  "next",
  "playing",
  "choose",
  "effect",
  "acknowledge",
  "ended",
  "invalid",
] as const;

export type Messages = Record<(typeof messageNames)[number], string>;
