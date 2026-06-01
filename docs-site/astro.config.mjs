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
            { label: "Overview", slug: "getting-started" },
            { label: "Install", slug: "getting-started/install" },
            { label: "First Scene", slug: "getting-started/first-scene" },
          ],
        },
        {
          label: "Guides",
          items: [
            { label: "Authoring Loop", slug: "guides/authoring-loop" },
            { label: "Localisation", slug: "guides/localisation" },
            { label: "Testing Dialogue", slug: "guides/testing-dialogue" },
          ],
        },
        {
          label: "Examples",
          items: [
            { label: "Overview", slug: "examples" },
            { label: "Headless CLI", slug: "examples/headless-cli" },
          ],
        },
        {
          label: "Adapters",
          items: [
            { label: "Overview", slug: "adapters" },
            { label: "Authoring Refresh", slug: "adapters/authoring-refresh" },
            { label: "Godot", slug: "adapters/godot" },
            { label: "Bevy", slug: "adapters/bevy" },
            { label: "Unity", slug: "adapters/unity" },
          ],
        },
        {
          label: "Reference",
          items: [
            { label: "Overview", slug: "reference" },
            { label: "CLI", slug: "reference/cli" },
            { label: "Rust API", slug: "reference/rust-api" },
            { label: "Benchmarks", slug: "reference/benchmarks" },
            { label: "Source Format", slug: "reference/source-format" },
            { label: "Schema", slug: "reference/schema" },
          ],
        },
        {
          label: "Migration",
          items: [{ label: "Overview", slug: "migration" }],
        },
        {
          label: "Release Notes",
          items: [{ label: "Overview", slug: "release-notes" }],
        },
      ],
    }),
  ],
});
