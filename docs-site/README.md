# Recite site

Hugo and the pinned, unmodified Hugo Book theme build the static site. The landing playground uses
the actual Rust compiler and runtime through wasm-bindgen. Site builds need Rust and the mise-pinned
WASM tools, but no Node runtime or Go compiler. Playwright browser tests and CSS/type checks use the
existing JavaScript toolchain; they are development dependencies.

From the repository root:

```sh
just web setup
just web dev
just web build
just web verify
just web test-browser
```

Use `mise exec --` before `just` if mise is not activated. `setup` installs the mise-pinned
generator and initializes the exact theme revision. Browser checks use the versioned Compose image;
`just quality setup-browsers` provisions the matching browsers for direct local Playwright runs.

Write ordinary Markdown in `content/`. Section pages use `_index.md`; frontmatter titles become
visible headings. Keep existing URLs and fragments when moving content. Hugo's language
configuration and `i18n/<locale>.toml` handle translated pages and whole interface messages. English
remains at the root. The landing scene comes from `fixtures/recite/valid/landing-junction.recite`;
visitors can edit it or open another single `.recite` file. Run lazily loads the real
compiler/runtime in a worker. Source stays in the browser. Next and choice buttons step the runtime;
effects are shown as requests, with explicit simulated completion for blocking effects. Game queries
have no registered host, so their normal runtime errors are visible. This is a single-file preview,
not an engine host; it compiles without a project schema.

`just web wasm` generates bindings before site builds and browser-image assembly. Keep wasm-bindgen
at the same version in `mise.site.toml` and the root Cargo dependencies. Generated bindings and
binaries are ignored, and Hugo fingerprints the module, binary and worker. A 64 KiB source limit, 64
MiB WASM linear-memory ceiling, 15-second execution watchdog, worker termination on Stop or edits,
and a 100-entry transcript bound keep this small example runner responsive. Loading has its own
60-second deadline. The linear-memory ceiling does not bound every allocation made by the browser.
Native tests exercise the bridge; browser tests cover actual WASM execution, editing, choices and
recovery.

The few templates in `layouts/` supply the landing page, visible titles, keyboard-accessible mobile
controls, and skip links. `assets/theme.js` adds a persistent light/dark/system preference.
Canonical identity assets are mounted from `assets/identity`; do not copy them into the site. Native
formatting and linting use the root quality commands and versioned configuration.

We replaced Astro/Starlight after comparing the full Markdown corpus and testing landing, First
Scene, and Alternatives: preserved URLs, search, no-JavaScript reading, narrow reflow, keyboard
controls and manual themes. The browser suite audits every published page in light/dark themes and
checks keyboard scrolling for both code blocks and tables. A native build took about 70 ms locally;
this is evidence from one machine, not a CI budget. The benefit is simpler build dependencies with
comparable authored template/style code. Svelte had no components or hydration and was removed.

Update Hugo Book by changing its submodule revision. Review upstream changes to the overridden
header/brand templates and run the browser suite. Keep upstream files unmodified. Reconsider
Starlight or another maintained shell if translated navigation or future interactive documentation
starts requiring substantial replacement of the theme's behavior. The generator language alone is
not a reason to maintain our own navigation or search framework.
