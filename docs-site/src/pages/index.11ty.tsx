import type { SiteData } from "../../tools/site.ts";
import english from "../content/i18n/en.json";
import { Layout } from "../components/Layout.tsx";
import { Landing } from "../components/Landing.tsx";

export const data = { permalink: "/" };
const page = { url: "/", title: english["landing.title"], description: english["landing.lead"] };

export default function ({ site }: { site: SiteData }) {
  return (
    <Layout page={page} chrome={site.chrome} scripts={[site.playgroundScript]}>
      <Landing playground={site.playground} />
    </Layout>
  );
}
