import { defineConfig } from "oxfmt";
import base from "../oxfmt.config.mts";

export default defineConfig({
  ...base,
  ignorePatterns: [
    ...(base.ignorePatterns ?? []),
    ".astro/**",
    ".browser-artifacts/**",
    "src/client/generated/**",
  ],
});
