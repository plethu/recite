# Rust candidate findings · 2026-09-07

Freya was selected on 2026-09-08; [decision](../../docs/decisions/gui-framework.md).
The comparison below records the evidence before that decision. **GPUI with GPUI Component is the
strongest challenger from this pass.** Floem carries a substantial accessibility
integration gap; Xilem/Masonry has promising text and accessibility foundations
but needs another host-integration pass. Slint remains a research baseline
because its distribution choices differ from Recite's MIT OR Apache-2.0 policy.
The framework choice is settled; no accessibility or platform milestone is closed.

GPUI has now advanced to the bounded whole-scene writer feature set: in-place
prose, speaker and destination controls, details, choices, guarded drafts,
applied history and runtime preview. Its matching light/dark views are in the
[visual review](visual-review.md#gpui). Floem and Xilem remain smaller feasibility
probes; their appearance should not be ranked as if implementation effort were equal.
None of these entries has manual writer-task acceptance yet.

## Results

| Candidate | Evaluated version | Build / native host | Text evidence | Disposition |
| --- | --- | --- | --- | --- |
| GPUI + GPUI Component | `gpui-pre` and platform 0.3.1; Component 0.6.0 | Builds; mapped at 1200 × 800 on isolated Hyprland Wayland workspace | Entity input protocol test covers marked Japanese composition, committed Unicode/newline, draft refusal, apply, Source/Script and stable IDs. Separate test checks the actual Recite grammar produces highlight styles. | Keep as Freya's principal challenger. |
| Floem | 0.2.0 | Builds; mapped at 1200 × 800 on isolated Hyprland Wayland workspace | Editor document mutation round-trips Unicode/newlines through Recite with stable IDs. Wrapping and IME hooks inspected in source; physical input and visual wrapping not accepted by this probe. | Park further UI polish pending a credible accessibility route. |
| Xilem / Masonry | 0.4.0 | Builds; native startup fails initializing the X11 clipboard on this machine | Headless Masonry TextArea test passes Japanese preedit/commit with Unicode/newline. | Retain as a reserve; repeat native evidence with the newer clipboard handling. |
| Slint | 1.17.1 metadata and API/licensing review | Not built or linked into Recite | Documentation/source assessment only | Keep as baseline unless its UI language and licensing choices are deliberately adopted. |

All macOS and Windows cells are untested. Component protocol tests do not prove
physical IME, BiDi interaction, clipboard integration, keyboard navigation or
screen-reader acceptance. The Floem test exercises its editor document, not a
rendered text-control interaction. Masonry's test exercises the underlying
widget rather than the full Xilem adapter.

## GPUI

This entry uses [GPUI Component 0.6](https://github.com/longbridge/gpui-kit), whose
matched GPUI dependencies are published as `gpui-pre` snapshots. It is not a
claim about the older standalone `gpui` 0.2.2 package. Each dependency is pinned
in the entry's own manifest and lockfile.

`TextareaState`/`Textarea` provide ordinary prose input; `EditorState`/`Editor`
provide source editing. Recite's existing generated tree-sitter grammar and
highlight query register through `LanguageRegistry`. This is a useful fit for
our Script/Source distinction without writing a text editor from primitives.
The tests verify marked composition exists and then commits correctly; they
do not inspect its pixels or involve an installed IME.

The Rust component syntax is direct and reasonably close to Freya's model.
Entities, update contexts, subscriptions and the separate platform launcher
add concepts to learn. The reusable controls come from a second project, so
maintenance includes the relationship between GPUI snapshots and Component.
Those are ownership considerations, not evidence of poor maintenance.

The advanced adapter now offers the same bounded writer controls as Freya.
Typing updates draft and preview freshness through input events. Commands refuse
unfinished IME composition before reading the input buffer, so Apply cannot
persist preedit. Navigation and text replacement reset the component buffer;
preview actions and attribute changes that leave text unchanged preserve it.
The component's pinned `set_value` implementation clears its local undo history;
physical shortcut behaviour still needs a host test.

The initial four GPUI tests cover: composition/Unicode and Source round-trip; real syntax
highlighting; speaker/destination edits, new choice IDs, applied undo/redo and
reactive preview staleness; and refusal to apply unfinished composition.
Build, formatting and Clippy with warnings denied pass. Independent review
caught the composition boundary and hidden error messages; both were fixed
and re-reviewed. No upstream changes were needed.

The controls and prose fit the accepted visual direction, though the current
layout is more button-heavy and shows less of the conversation than it should.
That is adapter work, not a demonstrated GPUI limitation. Freya remains my
provisional preference for this product; GPUI is now a credible alternative
with a useful ready-made Script/Source editing pair. Focus traversal, widget
undo shortcuts, physical IME, editable-text accessibility and macOS/Windows
remain unaccepted.

Reproduce this entry independently of the other candidate workspaces:

```sh
cargo test --locked --manifest-path prototypes/gui-bakeoff/candidates/gpui/Cargo.toml
cargo clippy --locked --manifest-path prototypes/gui-bakeoff/candidates/gpui/Cargo.toml --all-targets -- -D warnings
cargo build --locked --manifest-path prototypes/gui-bakeoff/candidates/gpui/Cargo.toml
python3 prototypes/gui-bakeoff/scripts/capture-gpui.py
```

## Floem

Floem's signal-driven API makes a small document editor straightforward. Its
published `TextEditor` exposes editor-width wrapping, gutter suppression,
custom font styling and document update callbacks. The prototype uses those
for Script, and its draft buffer uses Recite's existing edit contract.

The material concern is accessibility: the inspected 0.2.0 source and manifest
have no AccessKit/AT-SPI integration, and the upstream
[AccessKit issue remains open](https://github.com/lapce/floem/issues/8).
This is a larger commitment than adapting a text control or awaiting a bounded
IME fix. It cannot receive accessibility credit because Lapce uses it.

Current main's manifest was also inspected at
[`778bb5f`](https://github.com/lapce/floem/tree/778bb5f2aa08429e579ee2e6ac97e84fbf18b618).
It differs substantially from the published dependency graph; it was not built.
The release probe is not evidence that every current-main issue persists.
Source highlighting is not adapted in this entry, although the editor has a
custom styling interface. That omission belongs to our prototype.

## Xilem / Masonry

The published Xilem text input always wraps; the wrapper's lack of a wrap-mode
setter must not be mistaken for lack of wrapping. Its underlying Masonry
TextArea has wrapping, composition and accessibility code. The component test
confirms that the preedit notification does not leak Japanese composition into
authored text, and that committing Unicode/newlines produces the expected edit.
It may emit a change notification containing unchanged committed text during
preedit, so absence of all change notifications is not the right assertion.

Native startup failed in `masonry_winit::MasonryState::new`, at
`ClipboardContext::new().unwrap()`, because this environment's advertised X11
display refuses connections. The window's requested Wayland backend does not
remove that separate clipboard initialization. This is not a GPU/rendering
verdict, nor proof of failure on machines with working XWayland.

[Current main at `b81d8d7`](https://github.com/linebender/xilem/blob/b81d8d7a631849def6eeab282561439b963862e5/masonry_winit/src/event_loop_runner.rs)
already falls back to a no-op clipboard on Linux when initialization fails.
That source was inspected, not built. It avoids this startup panic but does not
establish functioning clipboard operations. No upstream issue was filed.

Xilem's ordinary Rust application state is appealing. Its current wrapper
requires more work for the writing surface's styling and source editor than
GPUI's supplied controls. The probe does not implement source highlighting or
true whole-scene in-place editing; these remain adapter work rather than
failures attributed to the toolkit.

## Slint

The published 1.17.1 crate declares GPL-3.0-only or Slint's royalty-free/software
license alternatives. Slint's [own licensing overview](https://slint.dev/get-started)
explains that application source can remain MIT/Apache while the combined GPL
work is distributed under GPLv3; its community alternative has separate terms
and attribution requirements. None is simply an MIT/Apache dependency choice.
This pass does not change Recite's distribution policy or adopt another license.

Slint also uses its own declarative UI language, including when embedded through
Rust's [`slint!` macro](https://docs.slint.dev/latest/docs/rust/slint/macro.slint).
It is not XAML, and disliking XAML does not automatically settle that preference.
Together, the language and distribution questions make it a weaker fit for the
current direction. No runtime or input claims were tested.

## Verification

All three candidate builds, formatting and strict Clippy checks pass. Four
focused tests pass: Floem buffer/model round trip, GPUI composition/app round
trip, GPUI Recite highlighting, and Masonry composition/commit. Repository Git
policy and test-organisation checks pass. Independent read-only review found
no blockers within this bounded scope. Production source and dependencies are
unchanged; no commits, pushes or upstream publications were made in this pass.

## Reproduce

Each candidate is an isolated workspace, with its own lockfile. From the repo
root, replace `gpui` with `floem` or `xilem`:

```sh
mise exec -- cargo run --locked --manifest-path prototypes/gui-bakeoff/candidates/gpui/Cargo.toml
mise exec -- cargo test --locked --manifest-path prototypes/gui-bakeoff/candidates/gpui/Cargo.toml
mise exec -- cargo clippy --locked --manifest-path prototypes/gui-bakeoff/candidates/gpui/Cargo.toml --all-targets -- -D warnings
mise exec -- cargo fmt --manifest-path prototypes/gui-bakeoff/candidates/gpui/Cargo.toml --check
```

For non-interactive native startup checks on Hyprland 0.56+:

```sh
python3 prototypes/gui-bakeoff/scripts/smoke-candidates.py
```

Build all three first. The script uses unique window identities, silent private
workspaces, fixed floating allocations and per-window background rendering. It
checks placement, then terminates its own process groups and disables its rules.
It never injects desktop input or switches the user's workspace. Results go to
[`candidates/startup-evidence.json`](candidates/startup-evidence.json); startup
logs live beside each candidate. A failed candidate is recorded as failed.

The next useful comparison is Freya versus GPUI at equivalent writer-task depth,
followed by physical input and accessibility evidence on the intended platforms.
There is not yet a result that justifies separate full native implementations.

## Interaction follow-up

The GPUI suite now has seven passing tests. The follow-up exercises mock
clipboard paste, widget undo across passage changes, rejected draft recovery,
both preview branches and prose Tab/Shift+Tab. The latter exposed a default
multiline indentation binding; the adapter now overrides it only in Script.
See the [interaction evidence](evidence.md#interaction-pass--2026-09-07) and
[short hands-on trial](interaction-pass.md). This narrows the remaining work
to installed-host behaviour and writing preference; it does not close those
gates.
