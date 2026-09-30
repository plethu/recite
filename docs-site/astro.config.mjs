import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";
import svelte from "@astrojs/svelte";

export default defineConfig({
  output: "static",
  integrations: [
    starlight({
      title: "Recite",
      // English stays at the root; translated pages can be added under their locale.
      locales: { root: { label: "English", lang: "en" } },
      customCss: ["./src/styles/site.css"],
      components: { PageTitle: "./src/components/PageTitle.astro" },
      logo: {
        light: "../assets/identity/recite-wordmark.svg",
        dark: "../assets/identity/recite-wordmark-reversed.svg",
        replacesTitle: true,
      },
      favicon: "/favicon.svg",
      social: [{ icon: "github", label: "GitHub", href: "https://github.com/plethu/recite" }],
      sidebar: [
        {
          label: "Getting Started",
          items: [
            { label: "Install", slug: "getting-started/install" },
            { label: "First Scene", slug: "getting-started/first-scene" },
          ],
        },
        {
          label: "Reference",
          items: [
            { label: "CLI", slug: "reference/cli" },
            { label: "Rust API", slug: "reference/rust-api" },
            { label: "Benchmarks", slug: "reference/benchmarks" },
            { label: "Source Format", slug: "reference/source-format" },
            {
              label: "Serialization Compatibility",
              slug: "reference/serialization-compatibility",
            },
          ],
        },
        {
          label: "Engine adapters",
          items: [
            { label: "Overview", slug: "adapters" },
            { label: "Edit and refresh", slug: "adapters/authoring" },
            { label: "Bevy", slug: "adapters/bevy" },
            { label: "Godot", slug: "adapters/godot" },
            { label: "Unity", slug: "adapters/unity" },
          ],
        },
        {
          label: "Guides and examples",
          items: [
            { label: "Complete CLI workflow", slug: "examples/headless-cli" },
            { label: "Authoring dialogue", slug: "guides/authoring-loop" },
            { label: "Localisation", slug: "guides/localisation" },
            { label: "Testing dialogue", slug: "guides/testing-dialogue" },
            { label: "Choosing dialogue tools", slug: "guides/alternatives" },
            { label: "Package preparation", slug: "guides/distribution" },
          ],
        },
        {
          label: "Migration",
          items: [
            { label: "Overview", slug: "migration" },
            { label: "Importer Boundaries", slug: "migration/importer-boundaries" },
            {
              label: "Dialogue System for Unity",
              slug: "migration/dialogue-system-for-unity",
            },
            { label: "Dialogue Manager", slug: "migration/dialogue-manager" },
            { label: "Dialogic", slug: "migration/dialogic" },
            { label: "Yarn Spinner", slug: "migration/yarn-spinner" },
            { label: "Ink", slug: "migration/ink" },
            { label: "Twee/Twine", slug: "migration/twee" },
            { label: "Clyde (manual)", slug: "migration/clyde" },
            {
              label: "JSON, CSV, and Engine-Native",
              slug: "migration/json-csv-engine-native",
            },
          ],
        },
      ],
    }),
    svelte(),
  ],
});
