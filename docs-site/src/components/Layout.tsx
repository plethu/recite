import type { Child } from "hono/jsx";
import type { SitePage } from "../../tools/content.ts";
import type { Assets } from "../../tools/assets.ts";
import type { Project } from "../showcase-schema.ts";
import { Header } from "./Header.tsx";

export interface Site {
  documents: Extract<SitePage, { kind: "document" }>[];
  projects: Project[];
  assets: Assets;
  source: string;
  wordmark: string;
}
// Apply before paint; storage can be unavailable in private/sandboxed contexts.
const themeScript = `try{const t=localStorage.getItem('theme');document.documentElement.dataset.theme=t==='light'||t==='dark'?t:matchMedia('(prefers-color-scheme:dark)').matches?'dark':'light'}catch{}`;

export function Layout({
  page,
  site,
  children,
}: {
  page: { title: string; description: string; url: string; kind: string };
  site: Site;
  children: Child;
}) {
  return (
    <html lang="en">
      <head>
        <meta charset="utf-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <title>{page.title} · Recite</title>
        <meta name="description" content={page.description} />
        {page.kind !== "missing" && (
          <link
            rel="canonical"
            href={new URL(page.url, process.env.SITE_URL ?? "http://localhost:4173").href}
          />
        )}
        <script dangerouslySetInnerHTML={{ __html: themeScript }} />
        <link rel="stylesheet" href={site.assets.css} />
        {["ibm-plex-sans-latin", "fraunces-latin"].map((face) => (
          <link
            rel="preload"
            href={`/fonts/${face}.woff2`}
            as="font"
            type="font/woff2"
            crossorigin="anonymous"
          />
        ))}
        <script type="module" src={site.assets.site} />
        {page.kind === "landing" && <script type="module" src={site.assets.controller} />}
      </head>
      <body>
        <a class="skip-link" href="#_top">
          Skip to content
        </a>
        <header class="site-header" data-pagefind-ignore>
          <Header wordmark={site.wordmark} />
        </header>
        <main id="content">{children}</main>
        <footer class="site-footer site-content" data-pagefind-ignore>
          Recite · MIT OR Apache-2.0 · <a href="https://github.com/plethu/recite">Source</a>
        </footer>
        <dialog id="search-dialog" aria-label="Search documentation" data-pagefind-ignore>
          <form method="dialog">
            <button aria-label="Close search">Close</button>
          </form>
          <p data-search-status role="status"></p>
          <label for="documentation-search">Search</label>
          <div id="search-results"></div>
        </dialog>
      </body>
    </html>
  );
}
