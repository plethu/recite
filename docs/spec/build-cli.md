# Scene manifests, compiler and CLI

Part of the [production specification](../recite-production-spec.md). These are
requirements; implementation and release readiness require evidence from code,
tests and the current GitHub milestone. Section numbers remain stable.

## 11. Scene Manifest

### 11.1 Purpose

Every project uses exactly one `recite.project.toml` manifest to connect
project-owned dialogue sources and compiled assets to game concepts without
embedding game-specific data in the dialogue DSL. Project commands and shared
authoring clients discover the nearest manifest from the opened path; a nearer
manifest wins even when it is malformed.

The manifest must declare `format_version = 1`. Its optional `[discovery]`
table controls source enumeration:

- `source_roots` is an ordered, project-relative list, defaulting to `["."]`;
- `excludes` is a project-root-relative slash glob list that augments the
  non-disableable built-ins for hidden components, `target`, `build`, `dist`,
  `out`, `generated`, `vendor`, and `node_modules`;
- absolute, parent, backslash, and negation patterns are invalid;
- canonical roots and source files must remain within the project, symlink
  directories are not traversed, and non-UTF-8 paths or source text are
  reported as incomplete coverage;
- duplicate canonical roots are errors; overlapping roots are allowed with a
  warning, and the first declared root owns a shared document;
- eligible files have the exact lowercase `.recite` suffix and receive
  project-relative slash `DocumentKey`s sorted deterministically.

The shared discovery report retains valid documents alongside typed
diagnostics and marks coverage complete or partial. CLI project commands fail
on partial coverage; LSP clients may retain usable documents while publishing
the diagnostics. The LSP owns URIs, versions, overlays, and protocol state;
filesystem and path semantics belong to the shared authoring configuration.

The scene entries remain the project-level connection between dialogue assets
and game concepts.

This mirrors the current need for scene IDs, presentation modes, participants, and cinematic paths.

### 11.2 Example

```toml
format_version = 1

[project]
content_set = "base"
version = "0.1.0"

[[scenes]]
id = "scene.small-talk"
presentation = "portrait_dialogue"
asset = "Dialogue/Compiled/dialogue.recitec"
block = "small_talk_start"
participants = ["hazel", "rhea"]

[[scenes]]
id = "scene.heart-to-heart"
presentation = "cinematic_cutscene"
asset = "Dialogue/Compiled/dialogue.recitec"
block = "heart_to_heart_start"
participants = ["hazel", "rhea"]
cinematic_scene = "Scenes/Dialogue/Cutscenes/HeartToHeartCutscene.tscn"
```

### 11.3 Manifest Validation

Validation must check:

- duplicate scene IDs;
- missing compiled assets;
- missing source assets where configured;
- unknown start blocks;
- missing participants;
- unknown participants where a speaker/actor registry exists;
- presentation mode requirements;
- duplicate scene/block pairs if project policy disallows them;
- stale compiled assets.

## 12. Compiler

### 12.1 Compilation

The compiler must:

- parse dialogue sources;
- resolve imports/includes;
- validate block references;
- validate IDs;
- validate conditions;
- validate effects;
- validate metadata;
- validate markup;
- emit compiled assets;
- embed source fingerprints;
- embed schema fingerprint;
- embed compiler version;
- preserve source map information for diagnostics.

### 12.2 Compiled Format

The v0 compiled asset format is a deterministic MessagePack document with a
decoded compact JSON inspection form for fixtures, debugging, and CLI tooling.
The MessagePack bytes are the runtime-facing asset; the JSON form is
non-authoritative and must be produced from the same structured model.

The checked-in [v0 compiled-wire synchronization matrix](../compiled-wire-synchronization.md)
maps this shape and its numeric tags to the typed model, explicit encoder and
decoder/validator, inspection projection, fixtures, and conformance checks.
The accepted retention and migration policy is recorded in the
[serialization compatibility decision](../serialization-compatibility.md).

v0 uses:

- `format_version = 0`;
- `compiler_compatibility_version = 0`;
- MessagePack as the primary `.recitec` encoding;
- compact JSON as a decoded inspection encoding, not as the shipped runtime
  asset;
- BLAKE3 as the default content fingerprint algorithm.

The compiler must serialize deterministic tables, not parser-shaped object
graphs. The v0 wire contract must preserve row order explicitly and must not
depend on unordered map iteration. Lookup data is encoded as sorted tables keyed
by stable IDs. Repeated metadata entries remain ordered rows, even when keys
repeat. Field ordering, table ordering, string encoding, numeric representation,
and fingerprint inputs must be stable across repeated compiles of identical
validated input.

#### v0 wire shape

Runtime assets encode all compound values as fixed-length MessagePack arrays,
not maps. The decoded compact JSON inspection form renders the same arrays as
objects with the field names below. JSON field names are for humans and tests;
MessagePack array positions are authoritative.

Scalar wire rules:

- IDs and paths encode as UTF-8 strings.
- Index newtypes encode as unsigned 32-bit integers.
- Ranges encode as `[start, len]`, where `start` is the table index's `u32`
  value and `len` is a `u32` count.
- Optional values encode as MessagePack nil or the present value.
- Fingerprints encode as `[algorithm, digest]`, where `digest` is binary bytes.
- Source spans encode as `[file, start_line, start_column, end_line,
  end_column]`; `end_line` and `end_column` are nil for point spans.
- `Value` encodes as `[tag, payload]`, with tags `0 = scalar` and `1 = array`.
- `ScalarValue` tags are `0 = string`, `1 = integer`, `2 = float`, and
  `3 = boolean`.

Top-level and row arrays use this field order:

- `CompiledDialogue`: `[header, default_block, sources, blocks, statements,
  match_arms, lines, choices, availability_reasons,
  condition_availability_reasons, speakers, metadata, effects, source_maps,
  block_lookup, line_lookup, choice_lookup]`.
- `CompiledAssetHeader`: `[format_version, compiler_compatibility_version,
  primary_encoding, inspection_encoding, compiler_version, asset_id,
  source_map_id, schema_fingerprint]`.
- `CompiledSourceFile`: `[path, fingerprint]`.
- `CompiledBlock`: `[id, source_file, statements, metadata, default_speaker,
  source_map]`.
- `CompiledStatement`: `[kind, source_map]`.
- `CompiledMatchArm`: `[pattern, statements, source_map]`.
- `CompiledLine`: `[id, source_text, speaker, metadata, source_map,
  authored_source_text, interpolation_bindings, plural_source_text,
  authored_plural_source_text]`. The final two values are optional and are
  appended to the pre-release v0 row for plural source support.
- `CompiledChoice`: `[id, source_text, metadata, requirement,
  requirement_source_text, availability_reason_override, target, echo,
  source_map, authored_source_text, interpolation_bindings]`.

Decoded v0 assets may also contain legacy interpolation rows from the original
5-field `CompiledLine` shape (`id, source_text, speaker, metadata, source_map`)
and 9-field `CompiledChoice` shape (the current choice fields through `echo`
and `source_map`). The canonical encoder preserves those shapes when their
compiled rows are marked `Legacy`; legacy rows retain literal placeholder text,
have no interpolation bindings or plural fields, and require authored and
decoded source text to match. Current rows keep their existing 9-field and
11-field bytes. This is compatibility with already-supported v0 rows and does
not require a format-version bump.
- `CompiledAvailabilityReason`: `[id, template_source_text]`.
- `CompiledConditionAvailabilityReason`: `[function, reason, args]`.
- `CompiledAvailabilityReasonArgBinding`: `[name, value]`.
- `CompiledAvailabilityReasonArgValue`: `[tag, payload]`.
- `CompiledSpeaker`: `[id]`.
- `CompiledMetadataEntry`: `[key, value, source_map]`.
- `CompiledEffect`: `[id, mode, function, args, source_map]`.
- `CompiledSourceMapEntry`: `[source_file, span]`.
- Lookup entries: `[id, index]`, sorted strictly ascending by ID.

Enum-like values encode as `[tag, payload]` unless the variant has no payload,
in which case the payload is nil. v0 tags are:

- asset encoding: `0 = MessagePack`;
- inspection encoding: `0 = CompactJson`;
- schema fingerprint: `0 = fingerprint`, `1 = no_schema`;
- statement kind: `0 = line`, `1 = prompt`, `2 = divert`, `3 = if`,
  `4 = match`, `5 = effect`, `6 = end`;
- match pattern: `0 = variant`, `1 = wildcard`;
- divert target: `0 = block`, `1 = end`;
- choice echo: `0 = none`, `1 = selected_text`, `2 = explicit_line`;
- effect mode: `0 = deferred`, `1 = immediate`, `2 = blocking`;
- condition expression: `0 = call`, `1 = and`, `2 = or`, `3 = not`;
- argument: `0 = identifier`, `1 = value`;
- availability reason argument value: `"ConditionArg"`, `"LiteralString"`,
  `"LiteralInt"`, `"LiteralFloat"`, or `"LiteralBool"`.

Presentation projection rows and projection-specific tags are not encoded in the
current v0 wire shape. A coordinated pre-release wire change may extend this
table and the encoder/decoder mirrors before the first v0 reader ships; after
that point the versioning policy below applies. The change must be treated as a
format decision with fixtures and compatibility evidence, not as an implicit
encoder-only addition.

The compiled `requirement` tree stores condition calls plus schema-derived
availability reason mappings for positive boolean condition leaves.
`requirement_source_text` stores the compiler's canonical expression text for
the full requirement and is used for `RequirementExpression` origins. The
compiled data must be self-contained: runtime traversal and adapters must not
require the original schema manifest or game code to recover reason IDs,
template source text, parameter definitions, bound argument values, source
condition identity, or the full requirement expression identity.
`availability_reasons` is the compiled reason table used for localisation,
trace output, and adapter export during traversal.
`condition_availability_reasons` maps condition functions to compiled reason
IDs and bound argument values so runtime traversal can emit unavailable-choice
reasons without reparsing the schema manifest.

v0 fixed array arity is not append-compatible. While the project is
pre-release, the v0 shape may still be corrected: until the first tagged
release publishes compiled assets to external consumers, wire-shape corrections
may land as a coordinated update of the writer, reader, and fixtures without a
version bump. From the first tagged release onward, field additions, removals,
reordering, tag changes, or semantic changes require a `format_version` or
`compiler_compatibility_version` change. A v0 reader must reject unexpected
array lengths, unknown tags, invalid indexes, malformed lookup order, and
algorithm-specific fingerprint length mismatches as malformed compiled assets.

Compiled assets must include:

- format version;
- compiler compatibility version;
- compiler version;
- primary encoding and inspection encoding identifiers;
- asset identity and source-map identity;
- source file table;
- source fingerprints;
- schema fingerprint, or an explicit no-schema marker;
- default block index;
- block table;
- statement table;
- match arm table;
- line table;
- choice table;
- availability reason table;
- condition availability reason table;
- speaker table;
- metadata table;
- effect table;
- source map table;
- sorted lookup tables for block IDs, line IDs, and choice IDs.

The runtime-facing contract must exclude rowan syntax nodes, parser recovery
state, malformed source state, comments that are not part of runtime semantics,
and traversal over the `recite-core` source AST. Syntax trees and source AST
values are compiler and tooling inputs only. Runtime traversal consumes compiled
tables, source maps, fingerprints, and compact lookup indexes.

Custom binary, FlatBuffers, Cap'n Proto, bincode, postcard, CBOR, and other
encodings remain possible future versions only under the
[serialization compatibility decision](../serialization-compatibility.md), when
artifact-specific benchmark evidence or adapter requirements justify them.
They must not be introduced as v0 alternatives after assets exist without a
format or compatibility version change.

### 12.3 Freshness

The compiler must embed enough data for tooling to detect stale compiled assets.

`recite check-fresh` must compare:

- current source fingerprints;
- current schema fingerprint;
- current compiler compatibility version;
- compiled asset embedded source fingerprints;
- compiled asset embedded schema fingerprint or no-schema marker;
- compiled asset embedded compiler compatibility version.

The v0 freshness comparison is content-based. Source and schema fingerprints are
algorithm-tagged binary digest values; the initial algorithm is BLAKE3. The
MessagePack asset stores digest bytes directly. The compact JSON inspection form
may render those bytes as stable lowercase hexadecimal text. A compiler version
change alone does not require recompilation unless the compiler compatibility
version changes or the writer changes any runtime-facing semantics.

## 13. CLI

The CLI is a core product surface.

Required commands:

```text
recite compile <path-or-project>
recite validate <path-or-project>
recite validate-project <project-root>
recite extract <path-or-project>
recite check-ids <path-or-project>
recite check-fresh <project-root>
recite check-markup <path-or-project>
recite check-metadata <path-or-project> --schema <schema>
recite watch <project-root>
recite run <asset> --block <block> --fixture <fixture>
recite trace <asset> --block <block> --fixture <fixture>
recite play <asset> --block <block> [--ui auto|tui|plain] [--keymap standard|vim]
  [--dialogue-locale <locale>] [--dialogue-catalog <locale=path>]...
```

`recite play` is an interactive REPL for writers and a reference consumer of
the shared preview loop (Milestone 3). Future commands include `recite
generate-bindings --schema <schema> --lang <lang>` once the schema and adapter
contracts stabilise; it is not part of the v1 CLI surface.

### 13.1 `compile`

Compiles source dialogue into a compiled asset.

Must fail on validation errors unless `--allow-warnings` only warnings are present.

### 13.2 `validate`

Validates dialogue source without writing compiled output.

Must report all recoverable diagnostics.

### 13.3 `extract`

Emits POT files.

Options:

- output path;
- domain split;
- include/exclude speaker names;
- include/exclude metadata localisable fields.

### 13.4 `check-ids`

Reports:

- missing IDs;
- duplicate IDs;
- IDs that do not match project naming policy;
- IDs present in translations but absent from source;
- source strings whose ID changed unexpectedly where history data is available.

### 13.5 `run`

Runs a dialogue scene headlessly with fixture data.

Useful for tests, CI, and writer review.

Must be able to:

- auto-select choices by ID or index;
- reject fixture selections of unavailable choices without advancing traversal;
- auto-acknowledge immediate/blocking effects;
- emit transcript;
- emit effect list;
- emit condition query trace;
- emit unavailable choice reason trees in machine-readable output.

`run` may preview translated dialogue content only when the fixture opts in with
`[dialogue].locale`. Dialogue catalog paths in fixtures are resolved relative to
the fixture file directory. Without `[dialogue].locale`, line and choice output
must remain source text, and the runtime session locale remains unset. Catalogs
without a dialogue locale are an error.

### 13.6 `trace`

Produces a deterministic execution trace including:

- lines;
- prompts;
- choices;
- choice availability, including hidden-vs-unavailable behavior and structured
  unavailable reason trees;
- conditions evaluated;
- condition results;
- effects emitted;
- blocking acknowledgements;
- final deferred effects.

Structured trace field names and machine values are stable English identifiers.
When fixture dialogue preview is configured, trace output includes the selected
dialogue locale and fallback chain as metadata, while line and choice records
keep both `source_text` and preview `text` fields so terminal source fallback
remains testable.

### 13.7 `play`

Interactive REPL for writers. Loads a compiled asset, starts a scene, prints or renders lines and prompts, accepts choice selections by ID or index, asks for condition results as `y`/`n`, and requires explicit acknowledgement of blocking effects. Useful for fast authoring iteration without standing up a game.

`play` is a live authoring surface, distinct from the deterministic fixture runner. `run` and `trace` remain scriptable commands driven by fixture data; `play` is allowed to prompt the author and maintain an interactive transcript.

The default `--ui auto` mode should use a TUI when stdin and stdout are interactive terminals, and should fall back to the line-oriented plain mode for pipes, CI, and accessibility tooling. `--ui tui` must fail clearly when no interactive terminal is available and suggest `--ui plain`. `--ui plain` must preserve the same runtime event flow as the TUI with line-oriented prompts and responses, and is the screen-reader- and script-friendly play surface.

Interactive UI preferences are user preferences, not project content. The
shared authoring configuration contract must use `$RECITE_CONFIG` when it is
set, then an OS-aware user configuration base chosen by a platform strategy
(for example, `etcetera::choose_base_strategy()`), rather than assembling
`$HOME` paths in each frontend. That strategy resolves to
`$XDG_CONFIG_HOME/recite/config.toml` or `~/.config/recite/config.toml` on Linux
and other XDG systems, `~/Library/Application Support/Recite/config.toml` on
macOS, and the user's `AppData` Recite configuration path on Windows. Missing
config uses defaults.
UI preferences must not be stored in `recite.project.toml`. Window geometry,
recent-file lists, and other shell state are user-owned and must not become
project semantics. Malformed UI config must not affect `run` or `trace`.

Initial UI config:

```toml
[ui]
locale = "en-US"        # Recite UI BCP-47 locale, or "system"
keymap = "standard"      # "standard" or "vim"
key_hints = "contextual" # "contextual", "compact", or "hidden"
color = "auto"           # "auto", "always", or "never"
contrast = "standard"    # "standard" or "accessible"

[play]
show_unavailable_choices = true

[writer]
confirm_exit = true
view = "script"         # "script", "map", or "source"
theme = "light"         # "light" or "dark"
monochrome = false
shortcut_hints = false # true keeps shortcut chips visible without holding a modifier
reduced_motion = false
zoom_to_pointer = true
```

Workspace shortcuts can be changed in **Settings → Keyboard shortcuts**. The
optional `[writer.shortcuts]` table uses action names such as `commands`, `save`,
`save_all`, `script`, `map`, `source`, `focus`, and `split`. Values use portable
chords such as `"Primary+Shift+P"` or `"F8"`; `Primary` means Command on macOS and
Ctrl elsewhere. An empty string disables an assignment. Missing entries use the
defaults. Duplicate assignments, unmodified character keys, and reserved editing
or navigation keys are rejected when loading or saving. Apply, Undo and Redo stay
with the focused editor. These bindings never enter project files.

The native writer's Vim navigation adds `:` for Commands (`w`, `wa`, and `q`
invoke Save, Save all, and Close), `/` for scene/beat search, `n`/`N` for matching
beats, and `gg`/`G` for search-list boundaries. `Ctrl+o`/`Ctrl+i` follow navigation
history; `Ctrl+w` followed by `h/j/k/l` moves between visible panes. Pending pane
sequences show their destinations and can be cancelled with Escape. Search fields
show INSERT/NORMAL; character navigation does not replace text insertion. The
navigation actions also accept alternative workspace chords in Keyboard shortcuts.

The native writer confirms routine closure unless `writer.confirm_exit` is false.
This preference never bypasses unsaved project protection. Writer presentation
preferences apply across scenes and are user-owned; the shared `ui.keymap`
selects Standard or Vim navigation. Project manifests never supply these values.
Shared user configuration
loading is read-only; explicit typed edits reload and validate the current file,
preserve unrelated settings and comments, and use cooperative locking and atomic
replacement. Frontends own the controls and error presentation, not configuration
paths or file-writing policy.

When `color = "auto"`, TUI color is disabled if `NO_COLOR` is present or `CLICOLOR=0`; otherwise color may be used. `color = "always"` enables TUI color regardless of those environment variables, and `color = "never"` disables TUI color. `contrast = "accessible"` selects a higher-contrast, color-vision-friendlier palette when color is enabled. Color must never be the sole carrier of meaning: selected choices keep a `>` marker, unavailable choices keep textual unavailable/reason text, condition rows keep `yes`/`no` labels, and prompt, effect, transcript, and footer labels remain visible without color.

When `show_unavailable_choices` is true, `play` should render unavailable choices as disabled and may show a compact primary reason. The full structured reason tree remains available through trace/test output and adapter conformance fixtures. When the setting is false, `play` may hide unavailable choices as a UI preference only; runtime prompt output and previous-prompt state are unchanged.

When dialogue localisation is enabled, `play`, preview, and engine-facing
runtime operations must receive a locale from the caller or fixture and must
not infer it from the host environment. Without one, source-text-only mode is
valid and the optional runtime/session locale remains `None`. The Recite UI
locale is a separate preference; `system` is allowed for it and resolves
through the deterministic UI fallback chain. It controls Recite-owned text
across the CLI/TUI, GUI, LSP, and editor extensions—pane titles, labels,
prompts, status, diagnostics, and human errors—through the shared Fluent
resource contract. It never controls dialogue line or choice translation for
`play`, `run`, or `trace`; those remain runtime/provider concerns (§9). There
is no `--ui-locale` flag.

Dialogue content preview for `play` is separately opt in:

```text
recite play <asset> --block <block> --dialogue-locale fr-FR \
  --dialogue-catalog fr-FR=locale/fr-FR.po
```

`--dialogue-catalog` is repeatable and accepts `LOCALE=PATH`. Catalog paths on
the `play` command line are resolved relative to the current working directory
unless absolute. Passing a dialogue catalog without `--dialogue-locale` is an
error. Missing or empty catalog translations fall back to source text through
the runtime locale-provider path; Recite-owned UI text remains on the Fluent UI
catalog path.

Locale fallback for Recite-owned UI text is deterministic: an explicit UI
locale, or the resolved system UI locale, then language-only locale, then
`en-US`. Missing or malformed non-default Fluent resources fall back to
`en-US`. The default `en-US` resources are test-gated and must be complete
across the CLI/TUI, GUI, LSP, and editor extensions.

The default keymap is `standard`: arrows move choices, printable keys enter a choice ID/index, Enter submits typed input or the highlighted choice, and Ctrl-C/Esc/`:q`/`:quit` quit cleanly. Vim mode is opt-in: choices start in normal mode, `j`/`k` and arrows move, `i` enters text input, `:` opens command mode, and Esc leaves insert/command/help before quitting at the root prompt. No required play action may be reachable only through arrow-key navigation: plain mode accepts choices by ID/index, condition answers by typed values, and blocking-effect acknowledgement by Enter/`ack`; TUI mode keeps typed choice ID/index entry in standard mode and insert-mode typed entry in vim mode.

The TUI should include:

- a transcript pane for lines, selected choices, effects, acknowledgements, and end state;
- a current prompt pane with visible choice indexes and stable choice IDs;
- a status/footer area showing the compiled asset, block, and available controls;
- a condition prompt accepting `y`/`n`;
- a blocking-effect panel showing mode, runtime effect ID, function, args, and Enter/`ack` acknowledgement.

`play` must not execute game-side effects. Immediate and blocking effects remain typed runtime requests emitted to the authoring surface.

The exact CLI TUI is not itself a serious-v1 acceptance gate. It must consume
the same shared preview operations as the standalone workbench, remain usable
in plain mode, and preserve the runtime's structured event and effect
boundaries. The shared preview loop and the workbench preview are v1 gates.

### 13.8 `watch`

Authoring build loop for source, schema, and project changes:

```text
recite watch <project-root>
```

`watch` observes the project manifest, dialogue source files, generated schema
manifest, and other compile inputs. On change, it validates the project and
rebuilds compiled assets using the same deterministic whole-project compiler as
`compile`. It should reuse `check-fresh` fingerprint semantics so generated
assets can be compared against current source and schema without editor- or
engine-specific state.

The expected authoring loop is:

1. edit source or schema inputs;
2. LSP reports live diagnostics;
3. on save, LSP/editor code actions may insert missing stable IDs without
   rewriting existing IDs;
4. `recite watch` validates and rebuilds compiled assets;
5. the engine adapter imports or refreshes those assets and restarts the scene
   or applies the adapter's documented active-session policy.

`watch` must not imply mid-session patch reload for v1. It is a fast rebuild
surface for authoring, CI-adjacent local checks, and editor/engine integration.

The authoring kernel must expose the watch/build result as structured state so
the GUI and editor integrations do not parse human CLI output. The result must
identify the affected project inputs, diagnostics, output assets, freshness
state, and whether an active session must be restarted according to the
adapter's declared policy.

### 13.9 Future: `generate-bindings`

Deferred past v1. Will generate typed host-language bindings (condition stubs,
effect records/enums, runtime conversions, test helpers, optional engine
event/signal wrappers) from schema once the schema and adapter contracts
stabilise. This direction has no current tracker owner; assign one before it is
promoted into planned work.
