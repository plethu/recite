import { defineConfig } from "oxlint";
import base from "../../oxlint.config.mts";

export default defineConfig({
  ...base,
  ignorePatterns: [...(base.ignorePatterns ?? []), "src/*.generated.js"],
});
