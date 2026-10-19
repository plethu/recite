# Recite site

Eleventy builds the site from Markdown and TSX. Hono renders TSX to HTML during the build; it adds
no browser framework. Bun runs the build and bundles browser modules and CSS. TypeScript 7 checks
the templates, tooling and browser code. Hosting needs only static files, without a Node server or
search service. Deployment is tracked in #231 and is not configured here.

From the repository root, prefix these commands with `mise exec --` unless mise is activated:

```sh
just web setup
just web dev
just web build
just web verify
just web test-browser
```

`setup` installs pinned tools and the frozen Bun workspace. `dev` combines Eleventy's content
watcher with Bun's module watcher so edits to imported TSX components also render fresh HTML.
`build` writes `dist/`, including a Pagefind index. `preview` serves that output locally; it is a
development tool. Browser checks use the versioned Compose image with Chromium, Firefox and WebKit.
Content tests use Bun’s native runner (`bun run js:test docs-site/tests`). Oxfmt, Oxlint and
Stylelint own formatting, lint and the OKLCH palette rule.

Write Markdown in `src/content/docs/`. Frontmatter requires `title` and `description`; unknown keys
fail the build with a filename and field. `index.md` defines a section. Keep existing URLs and
GitHub-style heading fragments. `tools/content.ts` validates and renders content. Each route family
has its own Eleventy template in `src/pages/`; documents and projects use their own pagination.
`tools/site.ts` prepares shared assets and reference navigation, while `tools/output.ts` writes
redirects, sitemaps and the search index. Reference pages include a sidebar; guides use a reading
layout. The same Markdown renderer supplies accessible code/table scrolling and Shiki syntax colors
through theme tokens. Maintained Markdown may contain HTML for language and direction metadata. It
is trusted repository content, not an untrusted publishing endpoint.

Showcase records belong under `src/content/showcase/`. Each needs a title, summary and HTTPS
`projectUrl`. Optional `image` and nonempty `imageAlt` must appear together. Drafts do not publish.
The empty state is intentional; do not add investigation fixtures as real projects.

The header, reference menu and reading content work without JavaScript. Theme controls use a small
pre-paint script and follow the OS when set to System. Search loads Pagefind's UI and index only
when opened. The index retains hyphens, underscores and colons for CLI flags and Recite terms;
browser tests exercise representative queries and retry after a failed load. Index only authored
content, excluding navigation and playground controls.

The playground uses `fixtures/recite/valid/landing-junction.recite` and Recite's real Rust compiler
and runtime. `just web wasm` generates the ignored bridge and binary; keep wasm-bindgen versions
aligned in `.config/mise/config.site.toml` and Cargo. The worker and WASM load on Run. Source stays
in the browser; resource limits, exact integers, cancellation and recovery have browser and Rust
coverage.

Canonical identity assets come from `assets/identity`. The shared print palette and self-hosted
fonts come from the Writer/site redesign. Keep colors in `src/styles/tokens.css`, explicit CSS
layers and native scopes. Font licenses and subset sources live in `public/fonts/FONTS.md`. English
interface messages live in `src/content/i18n/en.json`; additional locales require route, message,
search and script-layout support rather than an implicit fallback. Set `SITE_URL` to the public
origin when preparing deployment; the default is localhost.

Before changing generators or browser behavior, preserve routes, heading links, no-JavaScript
reading and playground limits. Run `just web verify` and the browser suite across pages, themes and
engines, then inspect desktop and narrow-screen renders. Do not retain parallel generator
implementations or disposable investigation output after a migration.
