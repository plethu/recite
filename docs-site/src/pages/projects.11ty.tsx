import type { Project as ProjectContent } from "../showcase-schema.ts";
import type { SiteData } from "../../tools/site.ts";
import { Layout } from "../components/Layout.tsx";
import { Project } from "../components/Project.tsx";

export const data = {
  pagination: { data: "site.projects", size: 1, alias: "project" },
  permalink: ({ project }: { project: ProjectContent }) => project.url,
};

export default function ({ project, site }: { project: ProjectContent; site: SiteData }) {
  const page = { title: project.title, description: project.summary, url: project.url };
  return (
    <Layout page={page} chrome={site.chrome}>
      <Project project={project} />
    </Layout>
  );
}
