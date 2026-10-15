import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    name: "site",
    environment: "node",
    include: ["docs-site/tests/**/*.test.ts", "tests/**/*.test.ts"],
  },
});
