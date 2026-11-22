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
    sections: sectionsOf(documents),
  };
}

/** A top-level content folder: its index page when it has one, and its pages for the sidebar. */
export interface Section {
  title: string;
  url?: string;
  links: NavigationLink[];
}

export function segmentOf(url: string) {
  return url.split("/")[1] ?? "";
}

// Each folder's pages in file order, its index first; a folder without an index is titled from
// its name.
function sectionsOf(documents: { title: string; url: string }[]) {
  const sections: Record<string, Section> = {};
  for (const { title, url } of documents) {
    const segment = segmentOf(url);
    const words = segment.replaceAll("-", " ");
    const section = (sections[segment] ??= {
      title: words.charAt(0).toUpperCase() + words.slice(1),
      links: [],
    });
    if (url === `/${segment}/`) {
      Object.assign(section, { title, url });
      section.links.unshift({ title, url });
    } else section.links.push({ title, url });
  }
  return sections;
}

export type SiteData = Awaited<ReturnType<typeof buildSite>>;
export interface NavigationLink {
  title: string;
  url: string;
}
