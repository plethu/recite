import type { DocumentContent } from "../../tools/content.ts";
import type { SiteData } from "../../tools/site.ts";
import { Layout } from "../components/Layout.tsx";
import { Document, type SectionHead } from "../components/Document.tsx";

export const data = {
  pagination: { data: "site.documents", size: 1, alias: "document" },
  permalink: ({ document }: { document: DocumentContent }) => document.url,
};

// The running head names the page's top-level section, linked when that section has an index.
function sectionHead(document: DocumentContent, documents: DocumentContent[]): SectionHead {
  const segment = document.url.split("/")[1] ?? "";
  const index = documents.find(({ url }) => url === `/${segment}/`);
  if (index) return { title: index.title, url: index.url };
  const words = segment.replaceAll("-", " ");
  return { title: words.charAt(0).toUpperCase() + words.slice(1) };
}

export default function ({ document, site }: { document: DocumentContent; site: SiteData }) {
  return (
    <Layout page={document} chrome={site.chrome}>
      <Document
        document={document}
        section={sectionHead(document, site.documents)}
        referenceLinks={site.referenceLinks}
      />
    </Layout>
  );
}
