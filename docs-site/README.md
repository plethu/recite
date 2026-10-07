# Recite site

The public site needs an authored landing page, runnable examples, how-tos, a project showcase and
reference documentation. It should not present every section as a documentation page.

The current implementation uses Hugo and an unmodified, pinned Hugo Book theme. Hugo is temporary;
the comparison below identifies finalists for its replacement. The landing runs the actual Rust
compiler/runtime through wasm-bindgen. Node supports browser tests and CSS/type checks, but is not
required for the current site build.

From the repository root:

```sh
just web setup
just web dev
just web build
just web verify
just web test-browser
```

Prefix commands with `mise exec --` when mise is not activated. `setup` installs the pinned tools
and theme. Browser checks use the versioned Compose image; `just quality setup-browsers` installs
matching browsers for direct Playwright runs.

Write Markdown in `content/`; `_index.md` defines a section. Preserve published URLs and fragments.
The scene fixture is `fixtures/recite/valid/landing-junction.recite`. `just web wasm` generates its
bridge before builds. Keep wasm-bindgen versions aligned in `mise.site.toml` and Cargo dependencies.
Generated bindings/binaries are ignored. Canonical identity assets come from `assets/identity`.

The playground loads lazily in a worker and keeps source in the browser. Its resource limits and
recovery behavior are covered by native and browser tests. The browser suite checks every published
page in light/dark themes, keyboard scrolling, and actual playground execution. Keep upstream themes
unmodified and review matching upstream views when updating an overridden template.

## Generator decision

The 2026-10-07 investigation built 19 alternative configurations across 15 generators and an Astro
7.3.5/Starlight 0.42.4 baseline, followed by matched complete-site prototypes. **Recommend Astro
with Starlight for reference pages and authored layouts for the landing, how-tos and showcase.**
Zola 0.23.6/DevLab 0.8.0 meets the site structure and remains the strongest native alternative, but
its pinned highlighter cannot consume the required centralized OKLCH tokens. Production remains Hugo
until the migration passes its own gates.

Screens tested real content, tables, offline search, hit/miss/hit queries, keyboard activation, WASM
execution, 390px reflow and sampled axe checks. DevLab, Maudit and Astro also proved native
TypeScript/worker bundling; finalists tested same-name paths and malformed-content recovery.

| Candidate                                                                    | Decision and concrete evidence                                                                                                                                      |
| ---------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [Zola + DevLab](https://codeberg.org/RiPetitor/devlab-theme), 0.23.6 / 0.8.0 | Native reserve: complete-site layouts, search, runner and responsive preview work. Giallo blocks token-based syntax colors.                                         |
| [Zensical](https://zensical.org/), 0.0.68                                    | Strong workflow, active Material maintainers; Rust/Python pipeline. Three critical search accessibility defects persist after initialization and interaction.       |
| [docs-gen](https://github.com/yhirose/docs-gen), 0.7.0                       | Compact reserve: runner/search/routes/errors work. Young single-maintainer project; mobile table reflow and search semantics need repairs.                          |
| [RustPress](https://github.com/ZenithInc/rust-press), 0.1.11                 | Reserve: runner/search/routes/reflow work. Search is JavaScript; WASM asset is a placeholder. MVP with contrast/scrolling gaps.                                     |
| [mdBook](https://github.com/rust-lang/mdBook), 0.5.4                         | Established, working runner/search; book shell needs landing customization and ARIA repairs. Our `.playground` class collides with its Rust Playground feature.     |
| [Maudit + Pagefind](https://github.com/bruits/maudit), 0.12.1 / 1.5.2        | Excellent native bundler; reject current silent same-name page overwrites, pathless frontmatter panics and maudfmt comment deletion. Docs shell remains site-owned. |
| [Guidebook](https://github.com/guide-inc-org/guidebook), 0.1.75              | Meaningful CI/tests, working first-load runner. Mandatory SPA breaks return navigation; embedded Japanese/CDN templates lack an override seam.                      |
| [Bamboo](https://github.com/matthewjberger/bamboo), 0.5.10                   | Runner works. Default CDN-dependent search fails offline; landmarks, contrast and reflow need repairs.                                                              |
| [Compositor](https://github.com/Lockyc/compositor), 0.7.0                    | Runner works. Needs Pagefind plus mobile focus/reflow repairs; no ownership advantage over DevLab.                                                                  |
| [Doctave](https://github.com/Doctave/doctave), 0.4.2                         | Search works; default sanitization strips runner HTML. Latest release is from 2022.                                                                                 |
| [mdzk](https://github.com/mdzk-rs/mdzk), 0.5.2                               | Runner/search work. Released generator dates to 2022; current main pursues another direction. Prefer maintained mdBook.                                             |
| [Marmite](https://github.com/rochacbruno/marmite), 0.4.2                     | Active, working runner/search; blog shell needs docs/reflow customization. Review AGPL asset distribution before adoption.                                          |
| [Cobalt](https://github.com/cobalt-org/cobalt.rs), 0.20.4                    | Established general SSG; runner executes. No supplied docs/search shell. Fixture source fidelity remains unresolved, not a proven generator defect.                 |
| [ssg](https://github.com/sebastienrousseau/static-site-generator), 0.0.66    | Search works; CSP transformation externalizes our JSON data script and breaks the runner.                                                                           |
| [Hauchiwa](https://github.com/kamoshi/hauchiwa), 0.22.1                      | Working typed build library; docs example is a custom application, strips runner HTML and supplies no search. More presentation code to own.                        |

Zola themes Goyo 0.7.1, Tanuki, Abridge and AdiDoks also ran the scene. The latter three searched
successfully but needed more reflow/landmark repairs. Goyo reproducibly leaves stale empty results
after a hit, miss, then repeated hit. Abridge's optional PWA was disabled. Goyo/Tanuki/AdiDoks used
Zola 0.22.1; DevLab/current Abridge require 0.23/Tera 2. Git revisions are archived.

Dioxus, Leptos, Rspress, Blades, mdserve and Statical received source/documentation screening only:
broader app frameworks, a Node/React docs framework, a simpler SSG, a preview server or pipeline
parts. They were not measured as complete searchable docs products.

## Tradeoffs and adoption gates

Both complete-site prototypes passed scene execution/recovery, repeated search and return navigation
in Chromium, Firefox and WebKit. Five page types passed three widths in both themes; reading and
native menus worked without JavaScript. Both preserved published routes, links and fragments and
excluded three drafts. Showcase records were labelled internal fixtures.

The 35-route audit found syntax-contrast failures on three Zola pages. Astro’s table-focus gap was
repaired through its native Sätteri pipeline; keyboard scrolling and contrast of previously clipped
cells passed follow-up checks. All supplied callout roles passed both themes. A shared-style repair
restored active-sidebar contrast; axe had classified its 1:1 contrast as incomplete. Review those
results too. These checks do not establish complete accessibility or visual acceptance.

Both use small adapters and upstream search/theme/navigation. Zola’s responsive docs popover needed
a few lines. Astro validates showcase fields through its content schema; Zola fails at template use
and needs an HTTPS guard. Both rejected malformed records and recovered. Keep themes unmodified.

Zola is established; MIT-licensed DevLab is younger and predominantly single-maintainer. Pin its
revision and keep overrides small. Zola’s EUPL build-tool license does not automatically license
authored content.

Authored CSS uses centralized OKLCH tokens, explicit layers, native scopes, logical sizing and
reading measures such as `min(70ch, 44rem)`. Popover/dialog supply native interaction behavior;
isolation needs a demonstrated stacking purpose. Astro’s Shiki CSS-variable theme uses those tokens
and the existing Recite grammar. Zola 0.23.6 pins Giallo 0.5.2: disposable extra-theme builds
accepted hex and rejected CSS variables and OKLCH. Do not rewrite numbered color classes or remove
highlighting to conceal the gap. Vendor styles contain fallback literals; authored values and the
selected palette follow the strict contract.

Three alternating full-corpus preview rounds used native tools. Zola’s `serve --debounce 100` and
esbuild CSS/TypeScript bundling gave about 275/329/365 ms Markdown/template/CSS feedback; Astro gave
888/156/107 ms. Both retained source after CSS changes and ran after TypeScript reloads. This
supersedes Zola’s earlier 1.35 s result with default debounce; no custom watcher was needed.

Landing HTML plus initially requested JS/CSS was about 29 KB gzip for Zola and 50 KB for Astro.
Astro’s stock idle-loaded search UI accounted for about 26.5 KB; images, search queries and shared
lazy WASM were excluded. Zola also built the full corpus in about 110 ms versus Astro’s reported 1.5
s. These local observations are neither browser latency nor CI budgets. Astro earns its added build
dependency through native token support, typed content and component editing; static hosting needs
no Node server or hydrated framework. Zola remains useful if those boundaries improve.

Mise owns tool versions. Native esbuild/Zola builds need no Node/Go runtime; mise’s Go backend
installs esbuild. Pinned djLint’s Tera profile handles Tera 2 templates; generic HTML formatting
damages typed attributes. Astro uses dprint. CSS passed standard Stylelint rules plus exclusive
OKLCH literals. Carry these versioned settings into migration checks.

Before landing a migration, preserve routes, fragments, landing presentation, identity assets and
playground limits. Pass the existing Chromium/Firefox/WebKit suite across all pages and both themes,
including lifecycle recovery, keyboard table scrolling and JavaScript-disabled reading. Exclude
internal showcase/standards fixtures from publication. Use pinned mise tools and versioned
format/lint configuration. Verify translated navigation/messages when adding another locale. Do not
fork navigation/search to make the choice work. Keep page presentation schema-backed rather than
inferred from URL exclusions; map the supplied callout roles to the same design tokens.

Reevaluate Maudit after safe paths, contextual errors and lossless formatting; docs-gen/RustPress
after accessibility repairs and demonstrated maintenance; Guidebook after navigation/extension
seams; Goyo after its search fix. Reevaluate Zola when highlighting supports semantic variables
without color-class rewriting, or the palette requirement changes. No upstream issue was posted.

Detailed inputs, configurations, revisions, harnesses and raw evidence are archived locally at
`.git/branch-archives/rec-206/site-candidates-2026-10-07.tar.gz` and `site-whole-2026-10-07.tar.gz`
in the same directory. Experiments stay outside maintained source; this guide preserves the portable
decision and its limits.
