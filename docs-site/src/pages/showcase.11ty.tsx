import type { SiteData } from "../../tools/site.ts";
import { Layout } from "../components/Layout.tsx";
import { Showcase } from "../components/Showcase.tsx";

export const data = { permalink: "/showcase/" };
const page = {
  url: "/showcase/",
  title: "Projects using Recite",
  description: "Scenes, tools and games made with the language.",
};

export default function ({ site }: { site: SiteData }) {
  return (
    <Layout page={page} chrome={site.chrome}>
      <Showcase projects={site.projects} />
    </Layout>
  );
}
