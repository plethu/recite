import type { DocumentContent } from "../../tools/content.ts";
import type { NavigationLink } from "../../tools/site.ts";
import type { Heading } from "../../tools/markdown.ts";
import { PageTitle } from "./PageTitle.tsx";
import { RenderedMarkdown } from "./RenderedMarkdown.tsx";

/** The section a page belongs to; `url` is absent when the section has no index page. */
export interface SectionHead {
  title: string;
  url?: string;
}

export function Document({
  document,
  section,
  referenceLinks,
}: {
  document: DocumentContent;
  section: SectionHead;
  referenceLinks: NavigationLink[];
}) {
  const article = <DocumentArticle document={document} section={section} />;
  if (document.section === "reference") {
    return (
      <div class="reference-layout site-content">
        <ReferenceNavigation links={referenceLinks} currentUrl={document.url} />
        {article}
      </div>
    );
  }
  return <div class="site-content">{article}</div>;
}

// A sheet, as in Writer's galley: a running head that carries the place, one display title and
// a folio at the foot.
function DocumentArticle({
  document,
  section,
}: {
  document: DocumentContent;
  section: SectionHead;
}) {
  const own = section.url === document.url;
  return (
    <article class="sheet reading-panel markdown-content" data-pagefind-body>
      <p class="running-head" data-pagefind-ignore>
        {section.url && !own ? <a href={section.url}>{section.title}</a> : section.title}
        {!own && (
          <>
            <span aria-hidden="true"> › </span>
            <span class="visually-hidden">, </span>
            {document.title}
          </>
        )}
      </p>
      <PageTitle>{document.title}</PageTitle>
      <TableOfContents headings={document.headings} />
      <RenderedMarkdown html={document.html} />
      <p class="folio" data-pagefind-ignore>
        <span class="folio-place">{document.url}</span>
        <a href="#_top">Back to top</a>
      </p>
    </article>
  );
}

function ReferenceNavigation({
  links,
  currentUrl,
}: {
  links: NavigationLink[];
  currentUrl: string;
}) {
  return (
    <aside class="reference-navigation">
      <button class="reference-toggle" type="button" popovertarget="reference-menu">
        Reference menu
      </button>
      <nav id="reference-menu" class="slip" popover="auto" aria-label="Reference">
        {links.map(({ url, title }) => (
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
