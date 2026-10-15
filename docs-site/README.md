# Recite site

The public site needs an authored landing page, runnable examples, how-tos, a project showcase and
reference documentation. It should not present every section as a documentation page.

Astro and Starlight build static HTML, CSS and browser modules. Authored Astro layouts supply the
landing, how-tos and showcase; Starlight owns the reference shell, search and theme controls. The
landing runs the Rust compiler/runtime through wasm-bindgen. Mise-managed Bun installs the frozen
workspace and runs builds; Node runs Vitest and Playwright. Hosting needs no Node server or hydrated
UI framework. The intended deployment is Cloudflare Workers Static Assets; no deployment is
configured.

From the repository root:

```sh
just web setup
just web dev
just web build
just web verify
just web test-browser
```

Prefix commands with `mise exec --` when mise is not activated. `setup` installs the pinned tools
and frozen workspace packages. Browser checks use the versioned Compose image; `just quality
setup-browsers` installs matching browsers for direct Playwright runs.

Content tests use the shared Vitest site project (`bun run js:test --project site`). Playwright
retains the browser, accessibility and WASM acceptance checks. Standalone Oxfmt/Oxlint
configurations inherit the root defaults and own the site's generated-file exclusions and
browser/worker globals. TypeScript 7 checks ordinary site TypeScript with the derived
`tsconfig.typescript.json`; Astro checks component templates using the TypeScript 6 compiler API.
Astro builds with upstream Vite, and the shared test runner uses standalone Vitest.

On 14 October 2026, five alternating local runs compared Node 26.12.0, Node LTS 24.22.0, Bun 1.4.3
and workerd 1.20261014.1 on a Ryzen 7 9800X3D/Linux host. Packages and WASM were prepared first;
builds removed `dist` but reused filesystem and Astro caches. Static serving used the built landing
HTML, declared assets and the 2.13 MB playground WASM, with exact response hash checks.

| Runtime              |  Median Astro build | Static requests/s, 16 clients | Warm server RSS |
| -------------------- | ------------------: | ----------------------------: | --------------: |
| Node 26.12.0         |             0.842 s |                         8,685 |        86.1 MiB |
| Node 24.22.0 LTS     |             0.898 s |                         8,522 |        84.2 MiB |
| Bun 1.4.3            |             0.785 s |                        11,439 |        22.6 MiB |
| workerd 1.20261014.1 | Not a build runtime |                         7,779 |        56.0 MiB |

The serving comparison uses Node streams, `Bun.file` and workerd's read-only disk service with a
warm OS page cache. It measures uncompressed HTTP/1.1 loopback traffic from a client on the same
machine, not saturation, TLS, CDN delivery or remote Worker cold starts. The earlier RAM-cached
probe gave similar Node/Bun throughput; the native file-serving path accounts for Bun's larger gain.
Cloudflare's production static-asset routing is a separate deployment capability, not reproduced by
this local workerd service. The playground still runs in the browser under every serving option.

Bun's build gain over current Node was about 7%, while native static serving improved about 32%. Bun
is now the pinned installation and build runtime. The benchmark did not compare package managers;
the installation choice also uses its frozen lockfile, isolated workspaces, explicit script
allow-list and release-age policy. Node already executes erasable TypeScript directly, and an
installed VS Code extension uses its host's Node runtime. Bun and Node emitted identical CSS,
content and WASM; search script hashes differed because Vite serializes a preload helper with
`Function.toString()`. Browser acceptance must remain part of any switch. Both builds passed all 52
existing browser checks in Chromium, Firefox and WebKit. CPU and heap profiles of separate
instrumented builds are retained under `target/site-runtime-profiles/`; their different heap formats
are not comparable allocation counts.

Reproduce with `python3 scripts/benchmarks/site-runtime.py --help`, supplying explicit runtime
binary paths and a fresh output directory. The driver forces Bun's runtime, records versions, CPU
and process memory, checks response hashes and alternates execution order. Raw results from this run
are local artifacts in `target/site-runtime-disk-2026-10-14/` and the earlier cached probe in
`target/site-runtime-2026-10-14-final/`. These observations are not CI budgets.

Write Markdown in `src/content/docs/`; `index.md` defines a section. Preserve existing URLs and
fragments. How-tos use `template: splash` for the reading layout; reference pages use the native
sidebar. Page presentation is schema-backed, with public pages opting in through `PublicPage`. The
scene fixture is `fixtures/recite/valid/landing-junction.recite`. `just web wasm` generates its
bridge before builds. Keep wasm-bindgen versions aligned in `mise.site.toml` and Cargo dependencies.
Generated bindings/binaries are ignored. Canonical identity assets come from `assets/identity`.

The playground loads lazily in a worker and keeps source in the browser. Its resource limits and
recovery behavior are covered by native and browser tests. The browser suite checks generated pages
in both themes, keyboard scrolling and playground execution. Keep Starlight unmodified and review
matching upstream components when updating an override. `just web check` also tests showcase
metadata: title, summary, HTTPS `projectUrl`, and paired optional `image`/`imageAlt`. Add real
records under `src/content/showcase/`; the empty state is intentional and investigation fixtures are
excluded.

Own colors in `src/styles/tokens.css`, using OKLCH exclusively. CSS uses explicit layers, native
scopes and intrinsic sizing; Stylelint enforces the palette rule. English messages in
`src/content/i18n/en.json` own custom interface keys. When adding a locale, configure Starlight,
translate complete messages, and verify navigation, long labels and reading without JavaScript. Set
`SITE_URL` to the intended public origin when preparing a deployment; local builds use localhost.

## Generator choice and reevaluation

Astro/Starlight supplies typed content, component editing and token-based syntax highlighting while
keeping search, navigation and theme behavior upstream. Keep overrides small. The 7 October 2026
comparison covered 19 configurations across 15 generators, then matched complete Astro and
Zola/DevLab sites with real WASM execution, existing routes, search, responsive layouts and keyboard
checks in Chromium, Firefox and WebKit.

Zola 0.23.6/DevLab 0.8.0 remains the strongest native alternative. It built faster and transferred
less initial code in those local probes, but its pinned Giallo 0.5.2 rejected CSS variables and
OKLCH. Astro earns its build dependency through the required palette, native content schemas and
component workflow. Those measurements are diagnostic observations, not CI budgets or universal UX
claims.

Reopen a candidate only when the relevant boundary changes:

| Candidate                                             | Reevaluation condition                                                                                                                  |
| ----------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| Zola/DevLab                                           | Semantic syntax colors work without rewriting generated color classes or weakening the OKLCH requirement                                |
| Maudit/Pagefind                                       | Safe output paths, contextual frontmatter errors and lossless formatting                                                                |
| docs-gen/RustPress                                    | Search accessibility and mobile reflow are repaired, with demonstrated maintainer engagement                                            |
| Guidebook                                             | Return navigation preserves the runner and templates expose an extension seam                                                           |
| Goyo                                                  | Repeated hit/miss/hit search no longer retains stale empty results                                                                      |
| Zensical                                              | Its search accessibility defects are resolved and the Rust/Python pipeline offers enough benefit                                        |
| mdBook, Bamboo, Compositor, Marmite, Cobalt, Hauchiwa | A complete landing/showcase/reference workflow reduces site-owned presentation and accessibility work; review Marmite's asset licensing |
| Doctave/mdzk                                          | Renewed maintenance and required runner behavior justify reconsideration                                                                |
| ssg                                                   | CSP processing preserves the playground data and runner contract                                                                        |

The full dated assessment, including tested versions, build/bundle observations and limitations, is
preserved in
[the pre-trim guide](https://github.com/plethu/recite/blob/6e32b614bd8c91a6616f02ec2991b7e300808129/docs-site/README.md).
Retrieve it locally with `git show 6e32b614:docs-site/README.md`. The detailed prototypes and raw
measurements exist only in the original investigator's local `.git/branch-archives/rec-206/`
archives (`site-candidates-2026-10-07.tar.gz` and `site-whole-2026-10-07.tar.gz`); they are not
checkout prerequisites or shared reproduction assets.

Before changing generators, preserve routes, fragments, identity, reading without JavaScript and
playground resource limits. Run the existing browser suite across all pages and both themes,
including keyboard table scrolling, repeated search, return navigation and worker recovery. Compare
editing feedback, output size, setup and the code we would maintain. Refresh maintenance and license
evidence before adoption; do not retain parallel site implementations after deciding.
