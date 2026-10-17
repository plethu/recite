import type { Project } from "../showcase-schema.ts";
export function Showcase({ projects }: { projects: Project[] }) {
  return (
    <div class="recite-showcase site-content" data-pagefind-body>
      <h1 id="_top" tabindex={-1} data-pagefind-meta="title">
        Projects using Recite
      </h1>
      <p class="intro">Scenes, tools and games made with the language.</p>
      {projects.length ? (
        <ul class="project-list">
          {projects.map((project) => (
            <li class="project">
              {project.image && (
                <img
                  src={project.image}
                  alt={project.imageAlt}
                  width={640}
                  height={480}
                  loading="lazy"
                />
              )}
              <div>
                <h2>
                  <a href={project.url}>{project.title}</a>
                </h2>
                <p>{project.summary}</p>
              </div>
            </li>
          ))}
        </ul>
      ) : (
        <p>No projects are featured here yet.</p>
      )}
    </div>
  );
}
