import { defineConfig, type OxlintConfig } from "oxlint";

export default defineConfig<OxlintConfig>({
  categories: { correctness: "error" },
  env: { node: true },
  rules: { "no-undef": "error" },
  ignorePatterns: ["**/node_modules/**", "**/target/**", "**/dist/**"],
});
