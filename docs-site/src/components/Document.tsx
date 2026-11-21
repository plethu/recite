import type { DocumentContent } from "../../tools/content.ts";
import type { NavigationLink, Section } from "../../tools/site.ts";
import type { Heading } from "../../tools/markdown.ts";
import { format, text } from "../content/messages.ts";
import { modeFor } from "./modes.ts";
import { PageTitle } from "./PageTitle.tsx";
import { RenderedMarkdown } from "./RenderedMarkdown.tsx";

export function Document({ document, section }: { document: DocumentContent; section: Section }) {
  const article = <DocumentArticle document={document} section={section} />;
  if (section.links.length < 2) return <div class="site-content">{article}</div>;
  return (
    <div class="section-layout site-content">
      <SectionNavigation section={section} currentUrl={document.url} />
      {article}
    </div>
  );
}

// The places above this page, in the header's vocabulary: its mode, then its folder when that
// is a different page. Section index pages that are their own mode have none.
function crumbsFor(document: DocumentContent, section: Section) {
  const mode = modeFor(document.url);
  const crumbs: { title: string; url?: string }[] = [];
  if (mode && mode.href !== document.url) crumbs.push({ title: mode.title, url: mode.href });
  if (section.url !== document.url && section.url !== mode?.href) crumbs.push(section);
  return crumbs;
}

// A sheet, as in Writer's galley: a running head that carries the place, one display title and
// a folio at the foot.
function DocumentArticle({ document, section }: { document: DocumentContent; section: Section }) {
  const crumbs = crumbsFor(document, section);
  const index = section.links.findIndex(({ url }) => url === document.url) + 1;
  return (
    <article
      class="sheet reading-panel markdown-content"
      data-pagefind-body
      data-pagefind-meta={`section:${section.title}`}
    >
      {crumbs.length > 0 && (
        <p class="running-head" data-pagefind-ignore>
          {crumbs.map(({ title, url }) => (
            <>
              {url ? <a href={url}>{title}</a> : title}
              <span aria-hidden="true"> › </span>
              <span class="visually-hidden">, </span>
            </>
          ))}
          {document.title}
        </p>
      )}
      <PageTitle>{document.title}</PageTitle>
      <TableOfContents headings={document.headings} />
      <RenderedMarkdown html={document.html} />
      <p class="folio" data-pagefind-ignore>
        <span>
          {format("sheet.place", { section: section.title, index, count: section.links.length })}
        </span>
        <a href="#_top">{text("sheet.top")}</a>
      </p>
    </article>
  );
}

// The folder's pages, as Writer's sidebar lists a scene's beats under the scene's name.
function SectionNavigation({ section, currentUrl }: { section: Section; currentUrl: string }) {
  return (
    <aside class="section-navigation">
      <button class="section-toggle" type="button" popovertarget="section-menu">
        {format("sheet.menu", { section: section.title })}
      </button>
      <nav id="section-menu" class="slip" popover="auto" aria-labelledby="section-menu-title">
        <p id="section-menu-title" class="menu-head">
          {section.title}
        </p>
        {section.links.map(({ url, title }: NavigationLink) => (
          <a href={url} aria-current={url === currentUrl ? "page" : undefined}>
            <span>{title}</span>
          </a>
        ))}
      </nav>
    </aside>
  );
}

function TableOfContents({ headings }: { headings: Heading[] }) {
  const sections = headings.filter((heading) => heading.level === 2);
  if (sections.length === 0) return null;
  return (
    <nav class="page-outline" aria-label="On this page" data-pagefind-ignore>
      <details>
        <summary>On this page</summary>
        <ol>
          {sections.map(({ id, text }) => (
            <li>
              <a href={`#${id}`}>{text}</a>
            </li>
          ))}
        </ol>
      </details>
    </nav>
  );
}
