import type { SiteData } from "../../tools/site.ts";
import { Layout } from "../components/Layout.tsx";
import { PageTitle } from "../components/PageTitle.tsx";

export const data = { permalink: "/404.html" };
const page = {
  url: "/404.html",
  title: "Page not found",
  description: "Find your way back to Recite.",
  noindex: true,
};

export default function ({ site }: { site: SiteData }) {
  return (
    <Layout page={page} chrome={site.chrome}>
      <div class="recite-article site-content">
        <PageTitle>Page not found</PageTitle>
        <p>
          This page does not exist. <a href="/">Return to Recite</a> or{" "}
          <a href="/getting-started/first-scene/">write your first scene</a>.
        </p>
      </div>
    </Layout>
  );
}
