import { text } from "../../content/messages.ts";

const example =
  ":: junction default\n\n> speaker=Technician @again\n  Again.\n\n? @plan\n  Unfold the survey plan.\n  -> plan";

export function LandingHero() {
  return (
    <>
      <div class="hero">
        <div>
          <h1 id="_top" data-pagefind-meta="title" tabindex={-1} data-print={text("landing.title")}>
            {text("landing.title")}
          </h1>
          <p class="lead">{text("landing.lead")}</p>
          <p class="actions">
            <a class="primary" href="#scene-heading">
              {text("playground.run")}
            </a>
            <a href="/getting-started/first-scene/">{text("landing.action.firstScene")}</a>
            <a href="/reference/source-format/">{text("landing.action.sourceFormat")}</a>
          </p>
        </div>
        <figure class="scene-leaf">
          <span class="halftone" aria-hidden="true" />
          <figcaption>junction.recite</figcaption>
          <pre>{example}</pre>
        </figure>
      </div>
      <aside class="note-band" aria-label={text("landing.release.label")}>
        <p class="release">{text("landing.release")}</p>
        <p class="release-links">
          <a href="/getting-started/install/">{text("landing.release.install")}</a>
          <a href="/guides/distribution/">{text("landing.release.packages")}</a>
        </p>
      </aside>
    </>
  );
}
