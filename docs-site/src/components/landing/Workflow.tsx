import { text } from "../../content/messages.ts";

const steps = [
  {
    title: text("landing.write.title"),
    body: text("landing.write.body"),
    href: "/guides/authoring-loop/",
    label: text("landing.write.link"),
  },
  {
    title: text("landing.check.title"),
    body: text("landing.check.body"),
    href: "/guides/testing-dialogue/",
    label: text("landing.check.link"),
  },
  {
    title: text("landing.integrate.title"),
    body: text("landing.integrate.body"),
    href: "/adapters/",
    label: text("landing.integrate.link"),
  },
];

export function Workflow() {
  return (
    <section aria-labelledby="paths-heading">
      <p class="eyebrow">{text("landing.path.kicker")}</p>
      <h2 id="paths-heading">{text("landing.path.title")}</h2>
      <div class="workflow">
        {steps.map(({ title, body, href, label }) => (
          <div>
            <h3>{title}</h3>
            <p>{body}</p>
            <a href={href}>{label}</a>
          </div>
        ))}
      </div>
    </section>
  );
}
