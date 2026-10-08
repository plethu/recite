import { satteri } from "@astrojs/markdown-satteri";
import starlight from "@astrojs/starlight";
import { defineConfig } from "astro/config";
import reciteGrammar from "../editors/vscode/syntaxes/recite.tmLanguage.json" with { type: "json" };
import keyboardAccess from "./src/markdown-keyboard-access.ts";

export default defineConfig({
  output: "static",
  site: process.env.SITE_URL ?? "http://localhost:4173",
  integrations: [starlight({
    title: "Recite",
    locales: { root: { label: "English", lang: "en" } },
    customCss: ["./src/styles/site.css", "./src/styles/adapter.css"],
    sidebar: [{ label: "Reference", items: [{ autogenerate: { directory: "reference" } }] }],
    components: {
      Header: "./src/components/Header.astro",
      ContentPanel: "./src/components/ContentPanel.astro",
      PageTitle: "./src/components/PageTitle.astro",
      MarkdownContent: "./src/components/MarkdownContent.astro",
      Footer: "./src/components/Footer.astro",
    },
    expressiveCode: false,
    disable404Route: true,
  })],
  markdown: {
    processor: satteri({ hastPlugins: [keyboardAccess] }),
    shikiConfig: { theme: "css-variables", langs: [{ ...reciteGrammar, name: "recite" }] },
  },
});
