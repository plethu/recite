import { file, Glob } from "bun";
import { resolve } from "node:path";
import matter from "gray-matter";
import * as v from "valibot";
import { showcaseSchema } from "../src/showcase-schema.ts";
import type { Project } from "../src/showcase-schema.ts";
import { createMarkdown } from "./markdown.ts";
import type { Heading } from "./markdown.ts";

const docsSchema = v.strictObject({
  title: v.pipe(v.string(), v.trim(), v.nonEmpty()),
  description: v.pipe(v.string(), v.trim(), v.nonEmpty()),
});
export type SitePage =
  | {
      kind: "document";
      url: string;
      title: string;
      description: string;
      html: string;
      headings: Heading[];
    }
  | {
      [K in "landing" | "showcase" | "missing"]: {
        kind: K;
        url: string;
        title: string;
        description: string;
      };
    }["landing" | "showcase" | "missing"];
export const redirects = ["dialogue-system-for-unity", "dialogue-manager", "dialogic", "clyde"].map(
  (name) => ({ from: `/migration/${name}/`, to: `/migration/manual/#${name}` }),
);

export function contentUrl(name: string, prefix = "") {
  if (!/^(?:[a-z0-9-]+\/)*[a-z0-9-]+\.md$/.test(name))
    throw new Error(`Unsafe content path: ${name}`);
  return `${prefix}/${name
    .replace(/(?:^|\/)index\.md$/, "")
    .replace(/\.md$/, "")
    .replace(/\/$/, "")}/`.replace(/\/+/g, "/");
}

async function sources(root: string) {
  const files: string[] = [];
  for await (const name of new Glob("**/*.md").scan({
    cwd: root,
    onlyFiles: true,
    followSymlinks: false,
  }))
    files.push(name);
  return Promise.all(
    files
      .filter((name) => name.endsWith(".md"))
      .sort()
      .map(async (name) => ({ name, ...matter(await file(resolve(root, name)).text()) })),
  );
}

export async function loadContent(root: string) {
  const [docs, showcase] = await Promise.all([
    sources(resolve(root, "src/content/docs")),
    sources(resolve(root, "src/content/showcase")),
  ]);
  const languages = [
    ...new Set(
      [...docs, ...showcase].flatMap(({ content }) =>
        [...content.matchAll(/^```([^\s`]+).*$/gm)]
          .map((match) => match[1]!)
          .filter((name) => name !== "text" && name !== "recite"),
      ),
    ),
  ];
  const markdown = await createMarkdown(languages);
  try {
    const documents = docs.map(({ name, data, content }) => {
      const metadata = validate(docsSchema, data, name);
      return {
        kind: "document" as const,
        ...metadata,
        url: contentUrl(name),
        ...markdown.render(content),
      };
    });
    const projects: Project[] = showcase.flatMap(({ name, data, content }) => {
      const metadata = validate(showcaseSchema, data, `showcase/${name}`);
      return metadata.draft
        ? []
        : [
            {
              ...metadata,
              url: contentUrl(name, "/showcase"),
              html: markdown.render(content).html,
            },
          ];
    });
    return { documents, projects };
  } finally {
    markdown.dispose();
  }
}

function validate<const T extends v.GenericSchema>(
  schema: T,
  data: unknown,
  file: string,
): v.InferOutput<T> {
  const result = v.safeParse(schema, data);
  if (!result.success)
    throw new Error(
      `${file}: ${result.issues.map((issue) => `${v.getDotPath(issue) ?? "metadata"}: ${issue.message}`).join("; ")}`,
    );
  return result.output;
}
