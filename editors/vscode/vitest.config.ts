import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    name: "vscode",
    environment: "node",
    include: ["editors/vscode/test/**/*.test.{ts,mjs}", "test/**/*.test.{ts,mjs}"],
    testTimeout: 30_000,
  },
});
