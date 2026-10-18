import type { Project } from "../showcase-schema.ts";
import { PageTitle } from "./PageTitle.tsx";

export function Showcase({ projects }: { projects: Project[] }) {
  return (
    <div class="recite-showcase site-content" data-pagefind-body>
      <PageTitle>Projects using Recite</PageTitle>
      <p class="intro">Scenes, tools and games made with the language.</p>
      <ProjectList projects={projects} />
    </div>
  );
}

function ProjectList({ projects }: { projects: Project[] }) {
  if (projects.length === 0) return <p>No projects are featured here yet.</p>;
  return (
    <ul class="project-list">
      {projects.map((project) => (
        <ProjectCard project={project} />
      ))}
    </ul>
  );
}

function ProjectCard({ project }: { project: Project }) {
  return (
    <li class="project">
      {project.image && (
        <img src={project.image} alt={project.imageAlt} width={640} height={480} loading="lazy" />
      )}
      <div>
        <h2>
          <a href={project.url}>{project.title}</a>
        </h2>
        <p>{project.summary}</p>
      </div>
    </li>
  );
}
