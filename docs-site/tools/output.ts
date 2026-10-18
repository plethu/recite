import { write } from "bun";
import { resolve } from "node:path";
import { createIndex, close } from "pagefind";
import { redirects } from "./content.ts";
import type { SiteData } from "./site.ts";

export async function writeSiteOutput(site: SiteData, directory: string, urls: string[]) {
  const { origin } = site.chrome;
  const locations = urls
    .map((url) => `<url><loc>${new URL(url, origin).href}</loc></url>`)
    .join("");
  await Promise.all([
    write(
      resolve(directory, "sitemap-0.xml"),
      `<?xml version="1.0" encoding="UTF-8"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">${locations}</urlset>`,
    ),
    write(
      resolve(directory, "sitemap-index.xml"),
      `<?xml version="1.0" encoding="UTF-8"?><sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"><sitemap><loc>${origin}/sitemap-0.xml</loc></sitemap></sitemapindex>`,
    ),
    write(
      resolve(directory, "_redirects"),
      redirects.map(({ from, to }) => `${from} ${to} 301`).join("\n") + "\n",
    ),
    write(resolve(directory, "examples/junction.recite"), site.playground.source),
    ...redirects.map(({ from, to }) =>
      write(
        resolve(directory, `.${from}`, "index.html"),
        `<!doctype html><html lang="en"><head><meta charset="utf-8"><meta http-equiv="refresh" content="0;url=${to}"><title>Moved</title><link rel="canonical" href="${to}"></head><body><a href="${to}">Continue to the migration guide</a></body></html>`,
      ),
    ),
  ]);
  await indexDocumentation(directory);
}

async function indexDocumentation(directory: string) {
  const { index, errors } = await createIndex({
    excludeSelectors: ["[data-pagefind-ignore]"],
    includeCharacters: "-_:",
  });
  if (!index) throw new Error(errors.join("\n"));
  try {
    checkErrors((await index.addDirectory({ path: directory })).errors);
    checkErrors((await index.writeFiles({ outputPath: resolve(directory, "pagefind") })).errors);
  } finally {
    await index.deleteIndex();
    await close();
  }
}

function checkErrors(errors: string[]) {
  if (errors.length) throw new Error(errors.join("\n"));
}
