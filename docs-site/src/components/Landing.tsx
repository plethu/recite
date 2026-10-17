import { text } from "../content/messages.ts";
import { Playground, type PlaygroundProps } from "./Playground.tsx";
import { LandingHero } from "./landing/Hero.tsx";
import { Workflow } from "./landing/Workflow.tsx";

const links = [
  { href: "/getting-started/", title: text("landing.navigation.manual") },
  { href: "/reference/", title: text("landing.navigation.reference") },
  { href: "/migration/", title: text("landing.navigation.migration") },
  { href: "/guides/distribution/", title: text("landing.navigation.packages") },
  { href: "https://github.com/plethu/recite", title: text("landing.navigation.source") },
];

export function Landing({ playground }: { playground: PlaygroundProps }) {
  return (
    <div class="recite-landing site-content" data-pagefind-body>
      <LandingHero />
      <section aria-labelledby="scene-heading">
        <p class="eyebrow">{text("landing.scene.kicker")}</p>
        <h2 id="scene-heading">{text("landing.scene.title")}</h2>
        <Playground {...playground} />
      </section>
      <Workflow />
      <ReadingSection
        headingId="localisation-heading"
        title={text("landing.localisation.title")}
        paragraphs={[text("landing.localisation.body1"), text("landing.localisation.body2")]}
        href="/guides/localisation/"
        label={text("landing.localisation.link")}
      />
      <ReadingSection
        headingId="fit-heading"
        title={text("landing.fit.title")}
        paragraphs={[
          text("landing.fit.body1"),
          text("landing.fit.body2"),
          text("landing.fit.body3"),
        ]}
        href="/guides/alternatives/"
        label={text("landing.fit.link")}
      />
      <nav class="site-links" aria-label={text("landing.navigation.label")}>
        {links.map(({ href, title }) => (
          <a href={href}>{title}</a>
        ))}
      </nav>
    </div>
  );
}

function ReadingSection({
  headingId,
  title,
  paragraphs,
  href,
  label,
}: {
  headingId: string;
  title: string;
  paragraphs: string[];
  href: string;
  label: string;
}) {
  return (
    <section class="copy" aria-labelledby={headingId}>
      <h2 id={headingId}>{title}</h2>
      {paragraphs.map((paragraph) => (
        <p>{paragraph}</p>
      ))}
      <a href={href}>{label}</a>
    </section>
  );
}
