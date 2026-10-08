import { z } from "astro/zod";

export const showcaseSchema = z.strictObject({
  title: z.string().min(1),
  summary: z.string().min(1),
  projectUrl: z.url().startsWith("https://"),
  draft: z.boolean().default(false),
  image: z.string().optional(),
  imageAlt: z.string().min(1).optional(),
}).refine(entry => Boolean(entry.image) === Boolean(entry.imageAlt), {
  message: "A showcase image and its alternative text must be supplied together.",
  path: ["imageAlt"],
});
