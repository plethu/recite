import type { Project as ProjectContent } from "../showcase-schema.ts";
import { PageTitle } from "./PageTitle.tsx";
import { RenderedMarkdown } from "./RenderedMarkdown.tsx";

export function Project({ project }: { project: ProjectContent }) {
  return (
    <div class="site-content">
      <div class="recite-showcase sheet" data-pagefind-body data-pagefind-meta="section:Showcase">
        <PageTitle>{project.title}</PageTitle>
        <article class="recite-article">
          <p>{project.summary}</p>
          <RenderedMarkdown html={project.html} />
          <p>
            <a href={project.projectUrl}>Visit the project</a>
          </p>
          <p>
            <a href="/showcase/">Back to the showcase</a>
          </p>
        </article>
      </div>
    </div>
  );
}
