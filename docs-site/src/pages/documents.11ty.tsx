import type { DocumentContent } from "../../tools/content.ts";
import type { SiteData } from "../../tools/site.ts";
import { Layout } from "../components/Layout.tsx";
import { Document } from "../components/Document.tsx";

export const data = {
  pagination: { data: "site.documents", size: 1, alias: "document" },
  permalink: ({ document }: { document: DocumentContent }) => document.url,
};

export default function ({ document, site }: { document: DocumentContent; site: SiteData }) {
  return (
    <Layout page={document} chrome={site.chrome}>
      <Document document={document} referenceLinks={site.referenceLinks} />
    </Layout>
  );
}
