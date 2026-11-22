import { text } from "../content/messages.ts";

/** Top-level modes, as in Writer's tab row. Learn owns every manual section outside Reference. */
export const modes = [
  {
    href: "/getting-started/",
    title: text("navigation.learn"),
    sections: ["/getting-started/", "/guides/", "/examples/", "/migration/", "/adapters/"],
  },
  { href: "/showcase/", title: text("navigation.showcase"), sections: ["/showcase/"] },
  { href: "/reference/", title: text("navigation.reference"), sections: ["/reference/"] },
];

export function modeFor(url: string) {
  return modes.find(({ sections }) => sections.some((section) => url.startsWith(section)));
}
