# GUI framework: Freya

Accepted by the Recite maintainer on 2026-09-08.

Use Freya for the standalone writer. Stop expanding the framework bake-off.
Keep GPUI as the documented fallback and retain the other probes as historical
evidence. A future port is allowed if a concrete blocker warrants it; speculative
portability layers are not part of the workbench.

The maintainer prefers Freya's appearance and Rust authoring style. The Linux
prototype supports the intended whole-scene writing flow, separate prose and
Source controls, guarded edits, stable IDs and runtime preview. The upstream
response to the bounded preedit report was encouraging. These observations
justify starting work; they do not establish full accessibility or platform support.

## Evidence and platform claims

| Candidate | Linux | macOS | Windows | Disposition |
| --- | --- | --- | --- | --- |
| Freya 0.5.0-rc.4 | Native startup/captures and component interaction tests | Unrun | Unrun | Selected implementation |
| GPUI pre/platform 0.3.1 + Component 0.6.0 | Native startup/captures and component interaction tests | Unrun | Unrun | Fallback |
| GTK 4.22.4 / Rust 0.11.4 | Native capture and widget evidence | Unrun | Unrun | Archived comparison |
| Floem 0.2 | Build/startup and buffer probe | Unrun | Unrun | Archived; accessibility integration gap |
| Xilem/Masonry 0.4 | Build/composition probe; native clipboard startup failure | Unrun | Unrun | Archived |
| Slint, Avalonia, Flutter, platform-native lanes | Research or proposed baselines only | Unrun | Unrun | No further bake-off work planned |

See the [findings](../../prototypes/gui-bakeoff/rust-candidates.md),
[visual comparison](../../prototypes/gui-bakeoff/visual-review.md) and
[evidence ledger](../../prototypes/gui-bakeoff/evidence.md) for exact limits.
A native startup or component test is not a supported-platform declaration.
No screen-reader, physical IME, BiDi, packaging or macOS/Windows acceptance
is inferred from this selection.

The maintainer explicitly ended the exhaustive candidate comparison. This
supersedes the earlier plan to implement every native/non-Rust lane. It does
not waive accessibility or platform acceptance for the eventual workbench, and
does not by itself close milestone 5 or its GitHub issues.

## Ownership and maintenance

Recite's parser, authoring kernel, configuration discovery, compiler and runtime
remain authoritative. The frontend owns interaction, presentation, focus and
file-session orchestration. Keep stable IDs and source preservation independent
of Freya widget state. A port would replace substantial UI work, but should not
replace language semantics.

Continue with the pinned RC until a deliberate upgrade is verified. The known
Source preedit display defect is assumed fixed for final 0.5 by maintainer
direction; the pinned RC is not claimed fixed. Upstream work remains paused;
there is no authorization here to publish new issues or patches.

Freya's fast development means upgrades require repeatable text, focus and
accessibility regressions. GPUI also carries coordination between its snapshots
and a separate component project; that is part of the fallback's maintenance cost.

## Reconsideration and next work

Reopen the toolkit choice for an unresolved accessibility or text-input blocker
on a required platform, an unworkable distribution dependency, or demonstrated
maintenance cost that prevents shipping. A different-looking screenshot is not
a reason to restart the comparison.

The first retained slice is project discovery, file selection, scene editing,
preview and explicit save. Preserve the existing visual language and writer-first
Script view. The current implementation remains in the isolated experimental
workspace while its file lifecycle is established; do not treat the whole
bake-off adapter as production-ready.

Before broader use, finish close protection and unsaved-draft recovery,
project-context/schema preview, per-project diagnostics, conflict recovery,
full keyboard routes, assistive technology and platform validation. Fluent UI
localisation and the complete milestone 6 contract remain required. Do not
replace these gates with the statement that Freya was selected.
