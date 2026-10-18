import type { SiteData } from "../../tools/site.ts";

export interface PageMetadata {
  title: string;
  description: string;
  url: string;
  noindex?: boolean;
}

// Apply before paint; storage can be unavailable in private/sandboxed contexts.
const themeScript = `
try {
  let theme = localStorage.getItem("theme");
  if (theme !== "light" && theme !== "dark") {
    const systemDark = matchMedia("(prefers-color-scheme: dark)").matches;
    theme = systemDark ? "dark" : "light";
  }
  document.documentElement.dataset.theme = theme;
} catch {}
`;
const fonts = ["ibm-plex-sans-latin", "fraunces-latin"];

export function DocumentHead({
  page,
  chrome,
  scripts,
}: {
  page: PageMetadata;
  chrome: SiteData["chrome"];
  scripts: string[];
}) {
  const canonical = page.noindex ? undefined : new URL(page.url, chrome.origin).href;
  return (
    <head>
      <meta charset="utf-8" />
      <meta name="viewport" content="width=device-width, initial-scale=1" />
      <title>{page.title} · Recite</title>
      <meta name="description" content={page.description} />
      {canonical && <link rel="canonical" href={canonical} />}
      {page.noindex && <meta name="robots" content="noindex" />}
      <script dangerouslySetInnerHTML={{ __html: themeScript }} />
      <link rel="stylesheet" href={chrome.stylesheet} />
      {fonts.map((face) => (
        <link
          rel="preload"
          href={`/fonts/${face}.woff2`}
          as="font"
          type="font/woff2"
          crossorigin="anonymous"
        />
      ))}
      <script type="module" src={chrome.script} />
      {scripts.map((src) => (
        <script type="module" src={src} />
      ))}
    </head>
  );
}
