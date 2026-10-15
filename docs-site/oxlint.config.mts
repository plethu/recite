import { defineConfig } from "oxlint";
import base from "../oxlint.config.mts";

export default defineConfig({
  ...base,
  ignorePatterns: [
    ...(base.ignorePatterns ?? []),
    ".astro/**",
    ".browser-artifacts/**",
    "src/client/generated/**",
  ],
  overrides: [
    ...(base.overrides ?? []),
    { files: ["**/*.astro"], globals: { Astro: "readonly" } },
    { files: ["src/client/**/*.ts"], env: { node: false, browser: true } },
    { files: ["src/client/worker.ts"], env: { node: false, browser: false, worker: true } },
    { files: ["tests/browser/**/*.ts"], env: { node: true, browser: true } },
  ],
});
