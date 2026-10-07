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
7.3.5/Starlight 0.42.4 baseline. **Zola 0.23.6 with DevLab 0.8.0 and Astro remain finalists.** The
documentation-focused screen favored DevLab; it does not establish a whole-site winner. Compare
authored landing/showcase layouts, how-to discovery and the docs shell together before choosing. No
recommendation has replaced production.

Screens used three real pages, a wide table, offline requests, positive/negative/repeated-positive
search, keyboard result activation, actual WASM execution, 390px reflow and sampled axe checks. Most
screens borrowed playground assets; DevLab, Maudit and Astro separately proved native
TypeScript/worker bundling. Finalists also exercised nested same-name pages, malformed frontmatter
and recovery. Search was driven through each tool's native events and result widgets.

| Candidate                                                                    | Decision and concrete evidence                                                                                                                                      |
| ---------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [Zola + DevLab](https://codeberg.org/RiPetitor/devlab-theme), 0.23.6 / 0.8.0 | Docs finalist: working shell, search, runner and native preview. Small TOC override fixes nested interactive elements.                                              |
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

DevLab built the existing 35 Markdown files and passed internal-link checks. First Scene passed
sampled axe/reflow checks in both three-page and full-corpus builds. Mobile menu keyboard opening,
Escape and focus restoration worked. This is not a corpus-wide accessibility audit.

Zola is established; MIT-licensed DevLab is younger and predominantly single-maintainer. Pin its
revision and keep overrides small. Zola's EUPL build-tool license does not automatically license
authored content. Native esbuild plus `zola serve` supplied preview without a custom watcher. Mise's
Go backend installs esbuild; the resulting build needs neither Go nor Node at runtime. The existing
pinned djLint **Tera profile** formatted/linted real HTML templates without changing component
semantics. Generic HTML formatting mishandles Tera 2 typed attributes; configure the template
profile explicitly. Production asset hashes changed correctly; static live reload serves new code
without re-rendering the HTML hash.

Astro has smoother component editing: about 95 ms versus DevLab's 1.35 s full reload here. Markdown
updates were about 0.87/1.37 s. Different TypeScript observation methods prevent a latency ratio.
Landing HTML plus initially requested JS/CSS was about 24/30 KB gzip respectively, excluding images,
deferred search and shared WASM. DevLab simplifies build dependencies; it does not beat Astro's
client payload or equal every aspect of its component DX.

Five alternating-order warm samples measured about 55 ms for DevLab with TypeScript bundling, 158 ms
for Maudit with warm Cargo/Pagefind, and 1.39 s for Astro with bundling/indexing. Shared WASM
compilation was excluded. These local build timings are neither browser latency nor CI budgets.

Before landing a migration, preserve routes, fragments, landing presentation, identity assets and
playground limits. Pass the existing Chromium/Firefox/WebKit suite across all pages and both themes,
including lifecycle recovery and JavaScript-disabled reading. Use pinned mise tools and versioned
format/lint configuration. Verify translated navigation/messages when adding another locale. Do not
fork navigation/search to make the choice work.

Reevaluate Maudit after path-safe IDs, contextual errors and lossless formatting; docs-gen/RustPress
after accessibility repairs and demonstrated maintenance; Guidebook after supported navigation and
template extension seams; Goyo after its search fix. Reconsider Astro if interactive docs need
richer composition or DevLab requires large behavior overrides.

Detailed inputs, configurations, revisions, harnesses and raw evidence are archived locally at
`.git/branch-archives/rec-206/site-candidates-2026-10-07.tar.gz`. Experiments stay outside
maintained source; this guide preserves the portable decision and its limits.
