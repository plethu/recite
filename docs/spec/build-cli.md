# Scene manifests, compiler and CLI

Part of the [production specification](../recite-production-spec.md). Section numbers remain stable.

## 11. Scene Manifest

### 11.1 Purpose

Every project uses exactly one `recite.project.toml` manifest to connect project-owned dialogue
sources and compiled assets to game concepts without embedding game-specific data in the dialogue
DSL. Project commands and shared authoring clients discover the nearest manifest from the opened
path; a nearer manifest wins even when it is malformed.

The manifest must declare `format_version = 1`. Its optional `[discovery]` table controls source
enumeration:

- `source_roots` is an ordered, project-relative list, defaulting to `["."]`;
- `excludes` is a project-root-relative slash glob list that augments the non-disableable built-ins
  for hidden components, `target`, `build`, `dist`, `out`, `generated`, `vendor`, and
  `node_modules`;
- absolute, parent, backslash, and negation patterns are invalid;
- canonical roots and source files must remain within the project, symlink directories are not
  traversed, and non-UTF-8 paths or source text are reported as incomplete coverage;
- duplicate canonical roots are errors; overlapping roots are allowed with a warning, and the first
  declared root owns a shared document;
- eligible files have the exact lowercase `.recite` suffix and receive project-relative slash
  `DocumentKey`s sorted deterministically.

The shared discovery report retains valid documents alongside typed diagnostics and marks coverage
complete or partial. CLI project commands fail on partial coverage; LSP clients may retain usable
documents while publishing the diagnostics. The LSP owns URIs, versions, overlays, and protocol
state; filesystem and path semantics belong to the shared authoring configuration.

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

The compiler parses sources, resolves imports/includes and validates block references, IDs,
conditions, effects, metadata and markup. Compiled assets retain source and schema fingerprints,
compiler version and source maps for diagnostics.

### 12.2 Compiled Format

The v0 compiled asset format is a deterministic MessagePack document with a decoded compact JSON
inspection form for fixtures, debugging, and CLI tooling. The MessagePack bytes are the
runtime-facing asset; the JSON form is non-authoritative and must be produced from the same
structured model.

The [shared wire registry](../../crates/recite-core/src/compiled/wire.rs) coordinates field counts
and numeric tags with the typed model, codecs, inspection projection, and focused fixtures. The
accepted retention and migration policy is recorded in the
[serialization compatibility decision](../serialization-compatibility.md).

The current `format_version` and `compiler_compatibility_version` are both `0`. BLAKE3 is the
default content fingerprint algorithm.

The compiler must serialize deterministic tables, not parser-shaped object graphs. The v0 wire
contract must preserve row order explicitly and must not depend on unordered map iteration. Lookup
data is encoded as sorted tables keyed by stable IDs. Repeated metadata entries remain ordered rows,
even when keys repeat. Field ordering, table ordering, string encoding, numeric representation, and
fingerprint inputs must be stable across repeated compiles of identical validated input.

#### v0 wire shape

Runtime assets encode all compound values as fixed-length MessagePack arrays, not maps. The decoded
compact JSON inspection form renders the same arrays as objects with the field names below. JSON
field names are for humans and tests; MessagePack array positions are authoritative.

Scalar wire rules:

- IDs and paths encode as UTF-8 strings.
- Index newtypes encode as unsigned 32-bit integers.
- Ranges encode as `[start, len]`, where `start` is the table index's `u32` value and `len` is a
  `u32` count.
- Optional values encode as MessagePack nil or the present value.
- Fingerprints encode as `[algorithm, digest]`, where `digest` is binary bytes.
- Source spans encode as `[file, start_line, start_column, end_line, end_column]`; `end_line` and
  `end_column` are nil for point spans.
- `Value` encodes as `[tag, payload]`, with tags `0 = scalar` and `1 = array`.
- `ScalarValue` tags are `0 = string`, `1 = integer`, `2 = float`, and `3 = boolean`.

Top-level and row arrays use this field order:

- `CompiledDialogue`: `[header, default_block, sources, blocks, statements, match_arms, lines,
  choices, availability_reasons, condition_availability_reasons, speakers, metadata, effects,
  source_maps, block_lookup, line_lookup, choice_lookup]`.
- `CompiledAssetHeader`: `[format_version, compiler_compatibility_version, primary_encoding,
  inspection_encoding, compiler_version, asset_id, source_map_id, schema_fingerprint]`.
- `CompiledSourceFile`: `[path, fingerprint]`.
- `CompiledBlock`: `[id, source_file, statements, metadata, default_speaker, source_map]`.
- `CompiledStatement`: `[kind, source_map]`.
- `CompiledMatchArm`: `[pattern, statements, source_map]`.
- `CompiledLine`: `[id, source_text, speaker, metadata, source_map, authored_source_text,
  interpolation_bindings, plural_source_text, authored_plural_source_text]`. The final two values
  are optional and are appended to the pre-release v0 row for plural source support.
- `CompiledChoice`: `[id, source_text, metadata, requirement, requirement_source_text,
  availability_reason_override, target, echo, source_map, authored_source_text,
  interpolation_bindings]`.

Decoded v0 assets may also contain legacy interpolation rows from the original 5-field
`CompiledLine` shape (`id, source_text, speaker, metadata, source_map`) and 9-field `CompiledChoice`
shape (the current choice fields through `echo` and `source_map`). The canonical encoder preserves
those shapes when their compiled rows are marked `Legacy`; legacy rows retain literal placeholder
text, have no interpolation bindings or plural fields, and require authored and decoded source text
to match. Current rows keep their existing 9-field and 11-field bytes. This is compatibility with
already-supported v0 rows and does not require a format-version bump.

- `CompiledAvailabilityReason`: `[id, template_source_text]`.
- `CompiledConditionAvailabilityReason`: `[function, reason, args]`.
- `CompiledAvailabilityReasonArgBinding`: `[name, value]`.
- `CompiledAvailabilityReasonArgValue`: `[tag, payload]`.
- `CompiledSpeaker`: `[id]`.
- `CompiledMetadataEntry`: `[key, value, source_map]`.
- `CompiledEffect`: `[id, mode, function, args, source_map]`.
- `CompiledSourceMapEntry`: `[source_file, span]`.
- Lookup entries: `[id, index]`, sorted strictly ascending by ID.

Enum-like values encode as `[tag, payload]` unless the variant has no payload, in which case the
payload is nil. v0 tags are:

- asset encoding: `0 = MessagePack`;
- inspection encoding: `0 = CompactJson`;
- schema fingerprint: `0 = fingerprint`, `1 = no_schema`;
- statement kind: `0 = line`, `1 = prompt`, `2 = divert`, `3 = if`, `4 = match`, `5 = effect`, `6 =
  end`;
- match pattern: `0 = variant`, `1 = wildcard`;
- divert target: `0 = block`, `1 = end`;
- choice echo: `0 = none`, `1 = selected_text`, `2 = explicit_line`;
- effect mode: `0 = deferred`, `1 = immediate`, `2 = blocking`;
- condition expression: `0 = call`, `1 = and`, `2 = or`, `3 = not`;
- argument: `0 = identifier`, `1 = value`;
- availability reason argument value: `"ConditionArg"`, `"LiteralString"`, `"LiteralInt"`,
  `"LiteralFloat"`, or `"LiteralBool"`.

Presentation projection rows and projection-specific tags are not encoded in the current v0 wire
shape. A coordinated pre-release wire change may extend this table and the encoder/decoder mirrors
before the first v0 reader ships; after that point the versioning policy below applies. The change
must be treated as a format decision with fixtures and compatibility evidence, not as an implicit
encoder-only addition.

Compiled availability data is self-contained: traversal and adapters must recover reason IDs,
template text, parameter definitions, bound arguments and origin identity without the schema
manifest or game code. `availability_reasons` stores templates; `condition_availability_reasons`
maps positive boolean condition leaves to reasons and bound arguments. `requirement_source_text`
stores the canonical full expression for `RequirementExpression` origins.

v0 fixed array arity is not append-compatible. While the project is pre-release, the v0 shape may
still be corrected: until the first tagged release publishes compiled assets to external consumers,
wire-shape corrections may land as a coordinated update of the writer, reader, and fixtures without
a version bump. From the first tagged release onward, field additions, removals, reordering, tag
changes, or semantic changes require a `format_version` or `compiler_compatibility_version` change.
A v0 reader must reject unexpected array lengths, unknown tags, invalid indexes, malformed lookup
order, and algorithm-specific fingerprint length mismatches as malformed compiled assets.

Traversal consumes compiled tables, source maps, fingerprints and lookup indexes. Parser syntax,
recovery state, source ASTs and non-semantic comments must not enter runtime assets. Alternative
encodings require artifact-specific evidence and the compatibility decision above; introducing one
after assets exist requires a format or compatibility version change.

### 12.3 Freshness

`recite check-fresh` compares current source fingerprints, schema fingerprint or no-schema marker,
and compiler compatibility version against their embedded counterparts.

The v0 freshness comparison is content-based. Source and schema fingerprints are algorithm-tagged
binary digest values; the initial algorithm is BLAKE3. The MessagePack asset stores digest bytes
directly. The compact JSON inspection form may render those bytes as stable lowercase hexadecimal
text. A compiler version change alone does not require recompilation unless the compiler
compatibility version changes or the writer changes any runtime-facing semantics.

## 13. CLI

The [CLI reference](../../docs-site/src/content/docs/reference/cli.md) owns command usage; `recite
--help` owns the complete flag surface. The contracts below describe observable behavior rather than
a second option inventory. `play` is interactive; fixture-driven `run` and `trace` remain
scriptable. Binding generation is deferred beyond v1.

### 13.1 `compile`

Compiles source dialogue into a compiled asset.

Validation errors prevent compiled output; warnings remain diagnostics.

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

Runs a scene headlessly from fixture data, selecting choices by ID or index and acknowledging
immediate/blocking effects. Selecting an unavailable choice must fail without advancing. Output must
include a transcript, effects, condition query trace and machine-readable unavailable reason trees.

`run` may preview translated dialogue content only when the fixture opts in with
`[dialogue].locale`. Dialogue catalog paths in fixtures are resolved relative to the fixture file
directory. Without `[dialogue].locale`, line and choice output must remain source text, and the
runtime session locale remains unset. Catalogs without a dialogue locale are an error.

### 13.6 `trace`

Produces a deterministic execution trace of events, condition queries/results, blocking
acknowledgements and final deferred effects. It preserves choice availability and structured reason
trees, distinguishing hidden choices from unavailable ones. The
[trace model](../../crates/recite-cli/src/runtime_fixture/trace/model.rs) owns field definitions.

Structured trace field names and machine values are stable English identifiers. When fixture
dialogue preview is configured, trace output includes the selected dialogue locale and fallback
chain as metadata, while line and choice records keep both `source_text` and preview `text` fields
so terminal source fallback remains testable.

### 13.7 `play`

Interactive preview for writers: load a compiled asset, start a scene, select choices by ID or
index, answer condition queries and explicitly acknowledge blocking effects. Unlike fixture-driven
`run` and `trace`, it prompts the author and retains an interactive transcript.

The default `--ui auto` mode should use a TUI when stdin and stdout are interactive terminals, and
should fall back to the line-oriented plain mode for pipes, CI, and accessibility tooling. `--ui
tui` must fail clearly when no interactive terminal is available and suggest `--ui plain`. `--ui
plain` must preserve the same runtime event flow as the TUI with line-oriented prompts and
responses, and is the screen-reader- and script-friendly play surface.

UI preferences and shell state are user-owned and must not enter `recite.project.toml` or project
semantics. [`recite-config`](../../crates/recite-config/src/lib.rs) owns platform paths, typed
defaults, validation and source-preserving edits: use `$RECITE_CONFIG` first, otherwise the shared
OS-aware configuration strategy. Missing config uses defaults; malformed UI config must not affect
`run` or `trace`. Loading is read-only. Explicit changes retain unrelated settings and comments and
refuse stale writes. The [Writer guide](../../apps/writer/guide.md#keyboard-and-preferences) owns
its controls.

The shared color and contrast preferences apply to the TUI. Color must never be the sole carrier of
meaning: retain the selected-choice marker, unavailable/reason text, condition `yes`/`no` labels,
and visible prompt, effect, transcript and footer labels.

When `show_unavailable_choices` is true, `play` should render unavailable choices as disabled and
may show a compact primary reason. The full structured reason tree remains available through
trace/test output and adapter conformance fixtures. When the setting is false, `play` may hide
unavailable choices as a UI preference only; runtime prompt output and previous-prompt state are
unchanged.

[Localisation §9](runtime-localisation.md#9-localisation) owns dialogue opt-in, fallback and its
separation from UI locale. `play` catalogue paths resolve relative to the current working directory
unless absolute; fixture paths resolve relative to the fixture file. Supplying a catalogue without
`--dialogue-locale` is an error. The CLI reference owns invocation examples. UI locale is a
preference, with no `--ui-locale` flag.

Locale fallback for Recite-owned UI text is deterministic: an explicit UI locale, or the resolved
system UI locale, then language-only locale, then `en-US`. Missing or malformed non-default Fluent
resources fall back to `en-US`. The default `en-US` resources are test-gated and must be complete
across the CLI/TUI, GUI, LSP, and editor extensions.

The default keymap is `standard`; vim mode is opt-in. No required action may depend on arrow-key
navigation: both keymaps retain typed choice ID/index entry, plain mode accepts typed condition
answers, and blocking effects accept Enter/`ack`.

The TUI should show the transcript, current prompt with choice indexes and stable IDs, asset/block
status and available controls, `y`/`n` condition input, and blocking-effect mode, request ID,
function and arguments.

`play` must not execute game-side effects. Immediate and blocking effects remain typed runtime
requests emitted to the authoring surface.

The exact CLI TUI is not itself a serious-v1 acceptance gate. It must consume the same shared
preview operations as the standalone workbench, remain usable in plain mode, and preserve the
runtime's structured event and effect boundaries. The shared preview loop and the workbench preview
are v1 gates.

### 13.8 `watch`

Authoring build loop for source, schema, and project changes:

```text
recite watch <project-root>
```

`watch` observes the project manifest, dialogue source files, generated schema manifest, and other
compile inputs. On change, it validates the project and rebuilds compiled assets using the same
deterministic whole-project compiler as `compile`. It should reuse `check-fresh` fingerprint
semantics so generated assets can be compared against current source and schema without editor- or
engine-specific state.

The [authoring loop](../../docs-site/src/content/docs/guides/authoring-loop.md) connects editor
feedback, builds and adapter refresh. `watch` must not imply mid-session patch reload for v1.

The authoring kernel must expose the watch/build result as structured state so the GUI and editor
integrations do not parse human CLI output. The result must identify the affected project inputs,
diagnostics, output assets, freshness state, and whether an active session must be restarted
according to the adapter's declared policy.

### 13.9 Future: `generate-bindings`

Deferred past v1: generate typed host-language bindings from schema once the schema and adapter
contracts stabilise.
