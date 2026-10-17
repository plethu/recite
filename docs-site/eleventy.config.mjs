import { rm } from "node:fs/promises";
import { file, write } from "bun";
import { resolve } from "node:path";
import { createIndex, close } from "pagefind";
import { buildAssets } from "./tools/assets.ts";
import { loadContent, redirects } from "./tools/content.ts";
import english from "./src/content/i18n/en.json" with { type: "json" };

export default function (config) {
  let site;
  config.addExtension("11ty.tsx", {
    key: "11ty.js",
    compile() {
      return async function (data) {
        return "<!doctype html>" + (await this.defaultRenderer(data));
      };
    },
  });
  config.addTemplateFormats("11ty.tsx");
  // Bun watches imported TS/TSX; Eleventy’s JS dependency parser cannot parse them.
  config.setWatchJavaScriptDependencies(false);
  config.addWatchTarget("./src/**/*");
  config.addWatchTarget("./tools/**/*");
  // Absolute external paths avoid remapping the content watcher’s working directory.
  config.addWatchTarget(resolve("../assets/identity/**/*"));
  config.addWatchTarget(resolve("../fixtures/recite/valid/landing-junction.recite"));
  config.addPassthroughCopy({ public: "/" });
  config.on("eleventy.before", async () => {
    await rm("dist", { recursive: true, force: true });
    const [{ documents, projects }, assets, source, wordmark] = await Promise.all([
      loadContent(process.cwd()),
      buildAssets(process.cwd()),
      file("../fixtures/recite/valid/landing-junction.recite").text(),
      file("../assets/identity/recite-wordmark.svg").text(),
    ]);
    site = {
      documents,
      projects,
      assets,
      source,
      wordmark,
      pages: [
        {
          kind: "landing",
          url: "/",
          title: english["landing.title"],
          description: english["landing.lead"],
        },
        ...documents,
        {
          kind: "showcase",
          url: "/showcase/",
          title: "Projects using Recite",
          description: "Scenes, tools and games made with the language.",
        },
        {
          kind: "missing",
          url: "/404.html",
          title: "Page not found",
          description: "Find your way back to Recite.",
        },
        ...projects.map((project) => ({
          kind: "project",
          ...project,
          description: project.summary,
        })),
      ],
    };
  });
  config.addGlobalData("site", () => site);
  config.on("eleventy.after", async () => {
    const origin = new URL(process.env.SITE_URL ?? "http://localhost:4173").origin;
    await write(
      "dist/sitemap-0.xml",
      `<?xml version="1.0" encoding="UTF-8"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">${site.pages
        .filter((page) => page.kind !== "missing")
        .map((page) => `<url><loc>${new URL(page.url, origin).href}</loc></url>`)
        .join("")}</urlset>`,
    );
    await write(
      "dist/sitemap-index.xml",
      `<?xml version="1.0" encoding="UTF-8"?><sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"><sitemap><loc>${origin}/sitemap-0.xml</loc></sitemap></sitemapindex>`,
    );
    await write(
      "dist/_redirects",
      redirects.map(({ from, to }) => `${from} ${to} 301`).join("\n") + "\n",
    );
    for (const { from, to } of redirects) {
      const directory = resolve("dist", `.${from}`);
      await write(
        resolve(directory, "index.html"),
        `<!doctype html><html lang="en"><head><meta charset="utf-8"><meta http-equiv="refresh" content="0;url=${to}"><title>Moved</title><link rel="canonical" href="${to}"></head><body><a href="${to}">Continue to the migration guide</a></body></html>`,
      );
    }
    await write("dist/examples/junction.recite", site.source);
    const { index, errors } = await createIndex({
      excludeSelectors: ["[data-pagefind-ignore]"],
      includeCharacters: "-_:",
    });
    if (!index) throw new Error(errors.join("\n"));
    try {
      const result = await index.addDirectory({ path: "dist" });
      if (result.errors.length) throw new Error(result.errors.join("\n"));
      const output = await index.writeFiles({ outputPath: "dist/pagefind" });
      if (output.errors.length) throw new Error(output.errors.join("\n"));
    } finally {
      await index.deleteIndex();
      await close();
    }
  });
  return { dir: { input: "src/pages", output: "dist" }, templateFormats: ["11ty.tsx"] };
}
