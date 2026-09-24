# GUI framework: Freya

Accepted by the Recite maintainer on 2026-09-08.

Freya is the standalone writer's framework. The framework comparison is closed.
The maintained application lives in [apps/writer](../../apps/writer/README.md).
Other framework implementations, comparison harnesses, and redundant reports
have been removed; their evidence remains in Git history.

Recite's parser, authoring kernel, configuration discovery, compiler, and runtime
remain authoritative. The frontend owns presentation, focus, interaction, and
file-session orchestration. A future port would replace the frontend without
replacing language semantics; no speculative portability layer is required.

The toolkit choice may be reconsidered for a concrete accessibility, text-input,
distribution, or maintenance blocker. The pinned Freya RC remains subject to
repeatable regression checks before upgrades. Selection does not establish
screen-reader, physical IME, BiDi, packaging, macOS, or Windows acceptance.
See the [writer acceptance checks and known limits](../../apps/writer/acceptance.md).

The GUI and accessibility milestones remain governed by their full contracts.
A working native window does not replace those acceptance gates.
