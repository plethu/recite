# Recite site

The site uses Astro and Starlight for static documentation. Svelte integration
is installed for a future demo backed by a compiled Recite fixture. The current
landing scene is an excerpt, not a browser implementation of the dialogue
runtime.

From the repository root, run `mise install` and `just web setup`, then
`just web dev` for the local server. `just web check` checks formatting, types,
components, and CSS without building; `just web verify` also builds the site
and checks its links. Use `just web build` and `just web preview` to inspect
the production output, or `just web fmt` to apply formatting. Run `just web`
to list the site commands. From `docs-site/`, use the same recipe names without
the `web` prefix, such as `just dev` or `just check`.

Run `just web test-browser` for the browser suite. It builds the pinned
Playwright image with Chromium, Firefox, and WebKit, serves the production
build inside the container, and writes reports to `docs-site/.browser-artifacts/`.
Pass Playwright filters after the recipe name, for example
`just web test-browser --project=webkit`.

## Internationalisation

English is served at the site root. Put translated Markdown or MDX in
`src/content/docs/<locale>/`, keeping the same relative path as the English
page. For example, `getting-started/first-scene.md` becomes
`cy/getting-started/first-scene.md`.

Custom landing-page prose and labels live in `src/content/i18n/en.json`. Keep
semantic keys stable when the wording changes, and translate whole sentences
or paragraphs rather than assembling them from fragments. Preserve the scene’s
speakers, directions, and player actions as distinct elements; leave source
syntax, commands, and API names unchanged.

Add a locale to the Starlight configuration when its translation is ready,
including translations for custom sidebar labels. Use `getRelativeLocaleUrl`
for internal links so navigation retains the selected locale. Check translated
pages with JavaScript disabled, at narrow widths and increased text sizes, and
in right-to-left layout where applicable.
