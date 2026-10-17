import { Layout } from "../components/Layout.tsx";
import { Landing } from "../components/Landing.tsx";
import { Showcase } from "../components/Showcase.tsx";
import type { SitePage } from "../../tools/content.ts";
import type { Project } from "../showcase-schema.ts";
import type { Site } from "../components/Layout.tsx";

type Page = SitePage | (Project & { kind: "project"; description: string });
export const data = {
  pagination: { data: "site.pages", size: 1, alias: "document" },
  permalink: ({ document }: { document: Page }) => document.url,
};

export default function ({ document: page, site }: { document: Page; site: Site }) {
  return (
    <Layout page={page} site={site}>
      {page.kind === "landing" ? (
        <Landing site={site} />
      ) : page.kind === "showcase" ? (
        <Showcase projects={site.projects} />
      ) : page.kind === "missing" ? (
        <div class="recite-article site-content">
          <h1 id="_top" tabindex={-1}>
            Page not found
          </h1>
          <p>
            This page does not exist. <a href="/">Return to Recite</a> or{" "}
            <a href="/getting-started/first-scene/">write your first scene</a>.
          </p>
        </div>
      ) : page.kind === "project" ? (
        <div class="recite-showcase site-content" data-pagefind-body>
          <h1 id="_top" tabindex={-1} data-pagefind-meta="title">
            {page.title}
          </h1>
          <article class="recite-article">
            <p>{page.summary}</p>
            <div dangerouslySetInnerHTML={{ __html: page.html }} />
            <p>
              <a href={page.projectUrl}>Visit the project</a>
            </p>
            <p>
              <a href="/showcase/">Back to the showcase</a>
            </p>
          </article>
        </div>
      ) : (
        <div
          class={
            page.url.startsWith("/reference/") ? "reference-layout site-content" : "site-content"
          }
        >
          {page.url.startsWith("/reference/") && (
            <aside class="reference-navigation">
              <button class="reference-toggle" type="button" popovertarget="reference-menu">
                Reference menu
              </button>
              <nav id="reference-menu" popover="auto" aria-label="Reference">
                {site.documents
                  .filter((entry) => entry.url.startsWith("/reference/"))
                  .map((entry) => (
                    <a href={entry.url} aria-current={entry.url === page.url ? "page" : undefined}>
                      {entry.title}
                    </a>
                  ))}
              </nav>
            </aside>
          )}
          <article class="reading-panel markdown-content" data-pagefind-body>
            <h1 id="_top" tabindex={-1} data-pagefind-meta="title">
              {page.title}
            </h1>
            {page.headings.length > 0 && (
              <nav class="page-outline" aria-label="On this page" data-pagefind-ignore>
                <details>
                  <summary>On this page</summary>
                  <ol>
                    {page.headings
                      .filter((heading) => heading.level === 2)
                      .map((heading) => (
                        <li>
                          <a href={`#${heading.id}`}>{heading.text}</a>
                        </li>
                      ))}
                  </ol>
                </details>
              </nav>
            )}
            <div dangerouslySetInnerHTML={{ __html: page.html }} />
          </article>
        </div>
      )}
    </Layout>
  );
}
