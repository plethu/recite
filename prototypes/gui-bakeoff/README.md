# Recite GUI bake-off

**Decision, 2026-09-08:** [Freya is selected](../../docs/decisions/gui-framework.md).
The bake-off is closed to further candidates. Other probes remain reference
material; GPUI is the fallback. The selected Freya entry now also has an early
[file-backed mode](freya-workbench.md). Historical evidence below is retained
with its original limits.

The first executable comparison is **Freya 0.5.0-rc.4 versus GTK 4** on Linux.
Both use the accepted [visual language](../../docs/gui-visual-language.md), the
same Alice scene, and the same Rust authoring model. This is a disposable
experiment for milestone 5, not a selected toolkit or the production workbench.

For Freya #2248, the [upstream handoff](freya-upstream-handoff.md) records the
reproduction, correction to our initial Input assessment, and paused fix work.

From the repository root:

```sh
mise exec -- cargo run --locked --manifest-path prototypes/gui-bakeoff/Cargo.toml -p recite-bakeoff-freya
mise exec -- cargo run --locked --manifest-path prototypes/gui-bakeoff/Cargo.toml -p recite-bakeoff-gtk
```

Run them separately to compare at the same window size. Builds use an isolated
workspace and lockfile; production dependencies are unchanged. GTK requires
GTK 4.22 development libraries and pkg-config. Freya requires its native Skia
build prerequisites, including clang, CMake, pkg-config and GTK 3 development
libraries. A working desktop display is required. This machine's Wayland
session works; its advertised X11 display did not accept connections.

Both entries now show the whole scene, with one passage edited in place and
its surrounding dialogue and choices still visible. Freya uses multiline
Input for Script and CodeEditor for Source; GTK uses TextView for both.

## Try the same task in each

1. Select Alice and rewrite her question. Apply the draft. Undo and redo it.
2. Change the speaker using the named controls. Open details to inspect the
   unchanged ID, then close them.
3. Select a choice, rewrite its wording, and change its destination to Garden.
4. Add a choice in Which Way. The existing Recite ID planner supplies its ID.
5. Open Source. Check highlighting, edit it, apply, and return through a passage
   button. Invalid source stays available for repair; it is never overwritten
   by a reconstructed script.
6. Try the scene, continue, and choose a reply. Change the document and check
   that the existing preview becomes stale. Start a new preview explicitly.
7. Repeat in Dark, then use only the keyboard. Check Tab escape from prose,
   focus after navigation, multiline text, selection, clipboard, and undo.

Changes live only in the session. There is no Save or project picker yet.
Apply/discard is an explicit experiment boundary before switching fields;
applied document history survives navigation. Widget-level undo across field
changes still needs a dedicated check. This interaction remains something to
evaluate with writers, not an accepted production constraint.

## What is shared

`crates/authoring` uses Recite's real parser/CST to locate prose and header
fields, preserves surrounding source bytes, and submits updated source to the
existing `AuthoringKernel` overlay. Its small guarded field-edit adapter is
experimental: the production kernel does not yet expose these general
structured-writing commands. There is no second parser or runtime.

Preview compiles the current source and uses `recite_runtime::PreviewSession`.
It retains a fixed compiled revision until restarted. Effects never execute
game code. Conditions, effects, nesting and plurals are outside this first
structured subset and require Source; runtime requests outside the simple
dialogue path stop explicitly. The controls use the fixture's two speakers;
schema-driven speaker discovery is still outstanding.

Both Source views use the repository's generated tree-sitter grammar and
highlight query. `crates/grammar` is the small C ABI binding; only that shim
permits the unsafe language-function declaration. Other prototype crates
forbid unsafe Rust. Tree-sitter remains syntax highlighting, not semantic
validation.

## Evidence and remaining gates

See [the evidence ledger](evidence.md) for tested versus untested behaviour.
The first pass concentrates on writing and editing. It does **not** yet meet
the complete visual baseline: resizable/collapsible panes, system appearance,
200% text scaling and narrow layouts remain open. The whole-scene reading and
in-place editing flow is implemented.
GTK currently uses TextView, not GtkSourceView.

The [other Rust candidate probes](rust-candidates.md) now cover Floem, GPUI
and Xilem/Masonry, with Slint reviewed as a baseline. macOS and Windows native
lanes remain queued. Avalonia and Flutter remain comparison
baselines. Freya is selected; neither #54 nor #123 is closed by this slice.

Verify the isolated workspace:

```sh
mise exec -- cargo fmt --manifest-path prototypes/gui-bakeoff/Cargo.toml --all --check
mise exec -- cargo clippy --manifest-path prototypes/gui-bakeoff/Cargo.toml --workspace --all-targets --locked -- -D warnings
mise exec -- cargo test --manifest-path prototypes/gui-bakeoff/Cargo.toml --workspace --locked
mise exec -- cargo build --manifest-path prototypes/gui-bakeoff/Cargo.toml --workspace --locked
RECITE_INTEGRATION_PR=1 scripts/check-git-policy.sh
```

The tests cover source preservation (including Unicode and CRLF), grammar
injection refusal, stale revisions, effective speaker metadata, choice IDs,
undo/draft recovery, and actual runtime choice traversal. Freya component tests additionally exercise
wrapping, Tab escape, composition
presentation in Script, and actual app draft/Source/Script interactions. A native
GTK widget test exercises the corresponding draft flow and focus restoration:

```sh
mise exec -- cargo test --locked --manifest-path prototypes/gui-bakeoff/Cargo.toml -p recite-bakeoff-gtk --test scene -- --ignored --test-threads=1
```

That test needs a display and is excluded from the ordinary headless run.
Physical IME and assistive-technology behaviour still require installed-host
evidence. The known CodeEditor preedit issue is assumed to be fixed for Freya
0.5 final, per the Recite maintainer's direction; the pinned RC is not claimed fixed.

Matching render captures and regeneration commands are in the
[visual review](visual-review.md).

The final [hands-on writing trial](interaction-pass.md) includes isolated launch
commands, a short shared task and a result form for Linux and macOS.
