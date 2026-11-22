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

export const searchMessageNames = ["loading", "failed", "empty", "one", "many"] as const;

export type SearchMessages = Record<(typeof searchMessageNames)[number], string>;
