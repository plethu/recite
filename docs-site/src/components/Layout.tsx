import type { Child } from "hono/jsx";
import type { SiteData } from "../../tools/site.ts";
import { Header } from "./Header.tsx";
import { DocumentHead, type PageMetadata } from "./DocumentHead.tsx";
import { SearchDialog } from "./SearchDialog.tsx";

interface LayoutProps {
  page: PageMetadata;
  chrome: SiteData["chrome"];
  scripts?: string[];
  children: Child;
}

export function Layout({ page, chrome, scripts = [], children }: LayoutProps) {
  return (
    <html lang="en">
      <DocumentHead page={page} chrome={chrome} scripts={scripts} />
      <body data-pagefind-ignore={page.noindex ? "all" : undefined}>
        <a class="skip-link" href="#_top">
          Skip to content
        </a>
        <header class="site-header" data-pagefind-ignore>
          <Header wordmark={chrome.wordmark} url={page.url} />
        </header>
        <main id="content">{children}</main>
        <footer class="site-footer" data-pagefind-ignore>
          <p>
            Recite · MIT OR Apache-2.0 · <a href="https://github.com/plethu/recite">Source</a>
          </p>
        </footer>
        <SearchDialog />
      </body>
    </html>
  );
}
