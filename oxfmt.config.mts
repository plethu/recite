import { defineConfig } from "oxfmt";

export default defineConfig({
  printWidth: 100,
  tabWidth: 2,
  singleQuote: false,
  semi: true,
  endOfLine: "lf",
  ignorePatterns: ["**/node_modules/**", "**/target/**", "**/dist/**"],
});
