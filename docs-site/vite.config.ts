import { defineConfig } from "vite-plus";

export default defineConfig({
  fmt: {
    svelte: true,
    ignorePatterns: ["dist/**", ".astro/**", ".svelte-check/**", "src/content/docs/**/*.md"],
  },
  lint: {
    options: { typeAware: true, typeCheck: true },
    ignorePatterns: ["dist/**", ".astro/**", ".svelte-check/**"],
  },
});
