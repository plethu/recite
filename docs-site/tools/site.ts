import { file } from "bun";
import { resolve } from "node:path";
import { buildAssets } from "./assets.ts";
import { loadContent } from "./content.ts";

export async function buildSite(root: string) {
  const [{ documents, projects }, assets, source, wordmark] = await Promise.all([
    loadContent(root),
    buildAssets(root),
    file(resolve(root, "../fixtures/recite/valid/landing-junction.recite")).text(),
    file(resolve(root, "../assets/identity/recite-wordmark.svg")).text(),
  ]);
  return {
    documents,
    projects,
    chrome: {
      wordmark,
      stylesheet: assets.css,
      script: assets.site,
      origin: new URL(process.env.SITE_URL ?? "http://localhost:4173").origin,
    },
    playground: {
      source,
      workerUrl: assets.worker,
      moduleUrl: assets.module,
      binaryUrl: assets.binary,
    },
    playgroundScript: assets.controller,
    referenceLinks: documents
      .filter((page) => page.section === "reference")
      .map(({ title, url }) => ({ title, url })),
  };
}

export type SiteData = Awaited<ReturnType<typeof buildSite>>;
export interface NavigationLink {
  title: string;
  url: string;
}
