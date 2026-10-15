import { defineConfig } from "oxlint";
import base from "../../oxlint.config.mts";

export default defineConfig({
  ...base,
  ignorePatterns: [...(base.ignorePatterns ?? []), "src/**"],
  overrides: [
    ...(base.overrides ?? []),
    {
      files: ["grammar*.js"],
      globals: Object.fromEntries(
        [
          "grammar",
          "seq",
          "choice",
          "repeat",
          "repeat1",
          "optional",
          "prec",
          "field",
          "alias",
          "token",
          "blank",
        ].map((name) => [name, "readonly" as const]),
      ),
    },
  ],
});
