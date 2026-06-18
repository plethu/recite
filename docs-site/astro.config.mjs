import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";

export default defineConfig({
  integrations: [
    starlight({
      title: "Recite",
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
            {
              label: "JSON, CSV, and Engine-Native",
              slug: "migration/json-csv-engine-native",
            },
          ],
        },
      ],
    }),
  ],
});
