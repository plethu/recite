import * as v from "valibot";

const nonempty = v.pipe(v.string(), v.trim(), v.nonEmpty());
export const showcaseSchema = v.pipe(
  v.strictObject({
    title: nonempty,
    summary: nonempty,
    projectUrl: v.pipe(v.string(), v.url(), v.startsWith("https://")),
    draft: v.optional(v.boolean(), false),
    image: v.optional(nonempty),
    imageAlt: v.optional(nonempty),
  }),
  v.forward(
    v.check(
      (entry) => Boolean(entry.image) === Boolean(entry.imageAlt),
      "A showcase image and its alternative text must be supplied together.",
    ),
    ["imageAlt"],
  ),
);

export type Project = v.InferOutput<typeof showcaseSchema> & { url: string; html: string };
