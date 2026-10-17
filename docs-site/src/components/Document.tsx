import type { DocumentContent } from "../../tools/content.ts";
import type { NavigationLink } from "../../tools/site.ts";
import type { Heading } from "../../tools/markdown.ts";
import { PageTitle } from "./PageTitle.tsx";
import { RenderedMarkdown } from "./RenderedMarkdown.tsx";

export function Document({
  document,
  referenceLinks,
}: {
  document: DocumentContent;
  referenceLinks: NavigationLink[];
}) {
  const article = <DocumentArticle document={document} />;
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

function DocumentArticle({ document }: { document: DocumentContent }) {
  return (
    <article class="reading-panel markdown-content" data-pagefind-body>
      <PageTitle>{document.title}</PageTitle>
      <TableOfContents headings={document.headings} />
      <RenderedMarkdown html={document.html} />
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
      <nav id="reference-menu" popover="auto" aria-label="Reference">
        {links.map(({ url, title }) => (
          <a href={url} aria-current={url === currentUrl ? "page" : undefined}>
            {title}
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
