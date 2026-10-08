import { docsLoader, i18nLoader } from "@astrojs/starlight/loaders";
import { docsSchema, i18nSchema } from "@astrojs/starlight/schema";
import { glob } from "astro/loaders";
import { z } from "astro/zod";
import { defineCollection } from "astro:content";
import english from "./content/i18n/en.json";
import { showcaseSchema } from "./showcase-schema";

// English owns the message keys; other locales may use Starlight's fallback.
const messages = Object.fromEntries(Object.keys(english).map(key => [key, z.string()])) as Record<
  keyof typeof english,
  z.ZodString
>;

export const collections = {
  docs: defineCollection({
    loader: docsLoader(),
    schema: docsSchema({
      extend: z.object({ presentation: z.enum(["public", "article"]).default("article") }),
    }),
  }),
  i18n: defineCollection({
    loader: i18nLoader(),
    schema: i18nSchema({ extend: z.object(messages).partial() }),
  }),
  showcase: defineCollection({
    loader: glob({ pattern: "**/*.md", base: "./src/content/showcase" }),
    schema: showcaseSchema,
  }),
};
