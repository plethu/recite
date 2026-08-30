# v0 compiled-wire synchronization matrix

This is the maintenance map for the v0 compiled MessagePack document. It does
not define a second format: `docs/recite-production-spec.md` §12.2 remains the
wire authority, and the shared constants in
`crates/recite-core/src/compiled/wire.rs` are the checked-in registry for the
numeric tags and arity assertions. Tag constants are consumed by both codec
halves; arity constants are primarily consumed by the focused wire-contract
tests, with a few generic tuple codec paths sharing them. They are not a
complete schema source from which both production codecs derive every tuple
length. The matrix makes the owners and evidence easy to find when a row or
enum changes.

The three checks have deliberately different jobs:

1. `recite-compiler/tests/asset/wire_contract.rs` decodes compiler output as
   an untyped MessagePack sequence and checks the fixed array shape against the
   shared arity assertions. This catches encoder shape drift without relying on
   the typed decoder mirror; it does not mean every production codec tuple
   declaration consumes the registry constant.
2. `recite-compiler/tests/asset/tag_surface.rs` compiles a source that reaches
   every v0 enum variant and round-trips it through the core decoder. This
   catches one-sided encoder/decoder or tag handling drift.
3. `recite-compiler/tests/asset/wire_golden.rs` pins the resulting bytes. A
   mirrored change is therefore visible as a golden diff and requires an
   explicit pre-release wire decision (or a version bump after the first tagged
   release).

## Fixed arrays and tuple payloads

Every row below is encoded as a fixed-length MessagePack array. The names in
the field column are the JSON inspection names; their order is the wire order.
`messagepack/validate` is the shared `CompiledDialogue` semantic authority. It
runs after structural/newtype wire conversion for decoded assets and before
canonical encoding; its mode preserves legacy decoded-row compatibility while
ensuring that the encoder emits the current-row contract.

| Wire value and fields, in order | Arity assertion registry | Encoder | Decoder / validator | Inspection and evidence | Authority |
| --- | --- | --- | --- | --- | --- |
| `CompiledDialogue`: `header`, `default_block`, `sources`, `blocks`, `statements`, `match_arms`, `lines`, `choices`, `availability_reasons`, `condition_availability_reasons`, `speakers`, `metadata`, `effects`, `source_maps`, `block_lookup`, `line_lookup`, `choice_lookup` | `V0_COMPILED_DIALOGUE_FIELDS` (17) | `core/src/compiled/messagepack/encode/root.rs` `MsgDialogue` (compiler delegates) | `core/src/compiled/messagepack/wire.rs` `MsgDialogue` (structural conversion); shared `messagepack/validate` semantic authority | `compiler/src/wire/inspection.rs` `json_dialogue`; valid fixture JSON snapshot; wire-contract and golden tests | §12.2 `CompiledDialogue` |
| `CompiledAssetHeader`: `format_version`, `compiler_compatibility_version`, `primary_encoding`, `inspection_encoding`, `compiler_version`, `asset_id`, `source_map_id`, `schema_fingerprint` | `V0_ASSET_HEADER_FIELDS` (8) | `MsgHeader` | structural conversion; shared header/version authority | `json_header`; tag-surface fixture | §12.2 `CompiledAssetHeader` |
| `CompiledSourceFile`: `path`, `fingerprint` | `V0_SOURCE_FILE_FIELDS` (2) | `MsgSourceFile` | structural conversion; shared source path checks | `json_dialogue.sources`; core-language fixture and golden | §12.2 `CompiledSourceFile` |
| `ContentFingerprint`: `algorithm`, `digest` | `V0_FINGERPRINT_FIELDS` (2) | `MsgFingerprint` | structural conversion; shared fingerprint/digest checks | `json_fingerprint`; core fingerprint tests | §12.2 fingerprint rules |
| `CompiledBlock`: `id`, `source_file`, `statements`, `metadata`, `default_speaker`, `source_map` | `V0_BLOCK_FIELDS` (6) | `MsgBlock` | structural conversion; shared block/index/range checks | `json_dialogue.blocks`; core-language fixture | §12.2 `CompiledBlock` |
| `CompiledStatement`: `kind`, `source_map` | `V0_STATEMENT_FIELDS` (2) | `MsgStatement` | structural conversion; shared statement/reference checks | `json_statement_kind`; tag-surface fixture | §12.2 `CompiledStatement` |
| `CompiledMatchArm`: `pattern`, `statements`, `source_map` | `V0_MATCH_ARM_FIELDS` (3) | `MsgMatchArm` | structural conversion; shared arm/range/source-map checks | `json_match_pattern`; tag-surface fixture | §12.2 `CompiledMatchArm` |
| `CompiledLine`: `id`, decoded `source_text`, `speaker`, `metadata`, `source_map`, authored `source_text`, `interpolation_bindings`, optional decoded `plural_source_text`, optional authored `plural_source_text` | `V0_LINE_FIELDS` (9) | `MsgLine` | structural `MsgLine` conversion; shared `validate_lines` interpolation/plural/index checks | `json_dialogue.lines`; core-language fixture and plural round-trip test | §12.2 `CompiledLine` |
| `CompiledChoice`: `id`, decoded `source_text`, `metadata`, `requirement`, `requirement_source_text`, `availability_reason_override`, `target`, `echo`, `source_map`, authored `source_text`, `interpolation_bindings` | `V0_CHOICE_FIELDS` (11) | `MsgChoice` | structural `MsgChoice` conversion; shared `validate_choices` interpolation/requirements/ranges | `json_dialogue.choices`; availability-reason fixture | §12.2 `CompiledChoice` |
| `CompiledAvailabilityReason`: `id`, `template_source_text` (`template` in the current model/inspection) | `V0_AVAILABILITY_REASON_FIELDS` (2) | `MsgAvailabilityReason` | structural conversion; shared ID uniqueness/reference checks | `json_dialogue.availability_reasons`; availability-reason fixture | §12.2 `CompiledAvailabilityReason` |
| `CompiledConditionAvailabilityReason`: `function`, `reason`, `args` | `V0_CONDITION_AVAILABILITY_REASON_FIELDS` (3) | `MsgConditionAvailabilityReason` | structural conversion; shared `validate_dialogue` function/reference checks | `json_dialogue.condition_availability_reasons`; availability-reason fixture | §12.2 `CompiledConditionAvailabilityReason` |
| `CompiledAvailabilityReasonArgBinding`: `name`, `value` | `V0_AVAILABILITY_REASON_ARG_BINDING_FIELDS` (2) | `MsgAvailabilityReasonArgBinding` | structural conversion; shared validator nonempty-name/value checks | `json_availability_reason_arg_binding`; availability-reason fixture | §12.2 `CompiledAvailabilityReasonArgBinding` |
| `CompiledSpeaker`: `id` | `V0_SPEAKER_FIELDS` (1) | custom `MsgSpeaker` serializer | structural tuple conversion; shared speaker ID checks | `json_dialogue.speakers`; core-language fixture | §12.2 `CompiledSpeaker` |
| `CompiledMetadataEntry`: `key`, `value`, `source_map` | `V0_METADATA_ENTRY_FIELDS` (3) | `MsgMetadataEntry` | structural conversion; shared validator key/value/source-map checks | `json_dialogue.metadata`; compact JSON snapshot and punctuation metadata test | §12.2 `CompiledMetadataEntry` |
| `CompiledEffect`: `id`, `mode`, `function`, `args`, `source_map` | `V0_EFFECT_FIELDS` (5) | `MsgEffect` | structural conversion; shared validator function/args/index/source-map checks | `json_dialogue.effects`; tag-surface fixture | §12.2 `CompiledEffect` |
| `CompiledSourceMapEntry`: `source_file`, `span` | `V0_SOURCE_MAP_ENTRY_FIELDS` (2) | `MsgSourceMapEntry` | structural conversion; shared source-file/span validation | `json_dialogue.source_maps`; source-span tests | §12.2 `CompiledSourceMapEntry` |
| `SourceSpan`: `file`, `start_line`, `start_column`, `end_line`, `end_column` | `V0_SOURCE_SPAN_FIELDS` (5) | custom `MsgSourceSpan` serializer | structural endpoint conversion; shared span ordering validation | `json_source_span`; source-span tests | §12.2 source spans |
| Lookup entry: `id`, `index` (for each of `block_lookup`, `line_lookup`, `choice_lookup`) | `V0_LOOKUP_ENTRY_FIELDS` (2) | `MsgLookupEntry` | structural conversion; shared sorted lookup/table consistency validation | `json_dialogue.*_lookup`; core lookup tests | §12.2 lookup entries |
| Range: `start`, `len` (statement, match-arm, choice, and metadata ranges) | `V0_RANGE_FIELDS` (2) | `MsgRange` | structural conversion; shared range bounds validation | `json_range`; wire-contract and core decoder tests | §12.2 ranges |
| Tagged enum/value pair: `tag`, `payload` (numeric enum/value tags below) | `V0_TAGGED_VALUE_FIELDS` (2) | `core/src/compiled/messagepack/encode/tags.rs` `serialize_tagged!` (compiler delegates) | `core/src/compiled/messagepack/tags.rs`; unknown-tag and payload-shape checks | `tagged_json`; tag-surface and golden tests | §12.2 enum-like values |
| `CompiledAvailabilityReasonArgValue`: `tag`, `payload` (string-tagged argument value) | `V0_TAGGED_VALUE_FIELDS` (2) | `MsgAvailabilityReasonArgValue` in `core/src/compiled/messagepack/encode/tables.rs` (compiler delegates) | `MsgAvailabilityReasonArgValueWrapper` structural visitor in `core/src/compiled/messagepack/wire.rs`; shared validator finite-float/value checks | `json_availability_reason_arg_binding`; literal-reason fixture, raw-tag guard, and literal-reason golden bytes | §12.2 availability-reason argument value |
| Condition call payload: `function`, `args` | `V0_CONDITION_CALL_FIELDS` (2) | `MsgConditionCall` | structural `MsgConditionCall` conversion; shared condition/argument validation | `json_condition_call`; tag-surface fixture | §12.2 condition expression payload |
| Prompt statement payload: `line`, `choices` | `V0_PROMPT_STATEMENT_PAYLOAD_FIELDS` (2) | `MsgStatementKind::Prompt` | structural prompt branch; shared prompt range validation | `json_statement_kind`; tag-surface fixture | §12.2 statement kind payload |
| If statement payload: `condition`, `then_statements`, `else_statements` | `V0_IF_STATEMENT_PAYLOAD_FIELDS` (3) | `MsgStatementKind::If` | structural if branch; shared condition/range validation | `json_statement_kind`; tag-surface fixture | §12.2 statement kind payload |
| Match statement payload: `scrutinee`, `arms` | `V0_MATCH_STATEMENT_PAYLOAD_FIELDS` (2) | `MsgStatementKind::Match` | structural match branch; shared condition/arm validation | `json_statement_kind`; tag-surface fixture | §12.2 statement kind payload |

`CompiledAvailabilityReason` currently names its model field `template`; the
wire and inspection contract call that value `template_source_text`. The
matrix records the shape, not a model rename; preserving compact JSON and
runtime behavior is part of this issue.

## Numeric tag families

The compiled enum/model type is the update point for the semantic variant. The
named constants are the only numeric wire values; both codec halves consume
the tag constants. A new variant must be represented in the model, encoder,
decoder, validator, tag-surface source, and this table together.

| Wire family and variants | Tag constants | Compiled enum/model update point | Encoder | Decoder / validator | Inspection and evidence | Authority |
| --- | --- | --- | --- | --- | --- | --- |
| Asset encoding: `MessagePack` | `V0_ASSET_ENCODING_MESSAGEPACK = 0` | `CompiledAssetEncoding` in `compiled/header.rs` | `MsgAssetEncoding` | same type in `messagepack/tags.rs`; nil-payload check | `json_asset_encoding`; tag-surface and golden | §12.2 asset encoding |
| Inspection encoding: `CompactJson` | `V0_INSPECTION_ENCODING_COMPACT_JSON = 0` | `CompiledInspectionEncoding` in `compiled/header.rs` | `MsgInspectionEncoding` | same type in `messagepack/tags.rs`; nil-payload check | `json_inspection_encoding`; tag-surface and golden | §12.2 inspection encoding |
| Schema fingerprint: `Fingerprint`, `NoSchema` | `V0_SCHEMA_FINGERPRINT_TAG_FINGERPRINT = 0`; `V0_SCHEMA_FINGERPRINT_TAG_NO_SCHEMA = 1` | `SchemaFingerprint` in `compiled/fingerprint.rs` | `MsgSchemaFingerprint` | same type in `messagepack/tags.rs`; payload-required/nil checks | `json_schema_fingerprint`; schema/no-schema fixtures | §12.2 schema fingerprint |
| Statement kind: `Line`, `Prompt`, `Divert`, `If`, `Match`, `Effect`, `End` | `V0_STATEMENT_TAG_LINE = 0`; `V0_STATEMENT_TAG_PROMPT = 1`; `V0_STATEMENT_TAG_DIVERT = 2`; `V0_STATEMENT_TAG_IF = 3`; `V0_STATEMENT_TAG_MATCH = 4`; `V0_STATEMENT_TAG_EFFECT = 5`; `V0_STATEMENT_TAG_END = 6` | `CompiledStatementKind` in `compiled/rows.rs` | `MsgStatementKind` | same type in `messagepack/tags.rs`; `validate_statement` | `json_statement_kind`; tag-surface, wire-contract, golden | §12.2 statement kind |
| Match pattern: `Variant`, `Wildcard` | `V0_MATCH_PATTERN_TAG_VARIANT = 0`; `V0_MATCH_PATTERN_TAG_WILDCARD = 1` | `CompiledMatchPattern` in `compiled/rows.rs` | `MsgMatchPattern` | same type in `messagepack/tags.rs`; match-arm validation | `json_match_pattern`; tag-surface and golden | §12.2 match pattern |
| Divert target: `Block`, `End` | `V0_DIVERT_TARGET_TAG_BLOCK = 0`; `V0_DIVERT_TARGET_TAG_END = 1` | `CompiledDivertTarget` in `compiled/rows.rs` | `MsgDivertTarget` | same type in `messagepack/tags.rs`; `validate_divert` | `json_divert_target`; tag-surface and golden | §12.2 divert target |
| Choice echo: `None`, `SelectedText`, `ExplicitLine` | `V0_CHOICE_ECHO_TAG_NONE = 0`; `V0_CHOICE_ECHO_TAG_SELECTED_TEXT = 1`; `V0_CHOICE_ECHO_TAG_EXPLICIT_LINE = 2` | `CompiledChoiceEcho` in `compiled/rows.rs` | `MsgChoiceEcho` | same type in `messagepack/tags.rs`; `validate_choice_echo` | `json_choice_echo`; tag-surface and golden | §12.2 choice echo |
| Effect mode: `Deferred`, `Immediate`, `Blocking` | `V0_EFFECT_MODE_TAG_DEFERRED = 0`; `V0_EFFECT_MODE_TAG_IMMEDIATE = 1`; `V0_EFFECT_MODE_TAG_BLOCKING = 2` | `CompiledEffectMode` in `compiled/rows.rs` | `MsgEffectMode` | same type in `messagepack/tags.rs` | `json_effect_mode`; tag-surface and golden | §12.2 effect mode |
| Condition expression: `Call`, `And`, `Or`, `Not` | `V0_CONDITION_TAG_CALL = 0`; `V0_CONDITION_TAG_AND = 1`; `V0_CONDITION_TAG_OR = 2`; `V0_CONDITION_TAG_NOT = 3` | `CompiledConditionExpression` in `compiled/rows.rs` | `MsgConditionExpression` | same type in `messagepack/tags.rs`; `validate_condition` | `json_condition_expression`; tag-surface and golden | §12.2 condition expression |
| Argument: `Identifier`, `Value` | `V0_ARGUMENT_TAG_IDENTIFIER = 0`; `V0_ARGUMENT_TAG_VALUE = 1` | `CompiledArgument` in `compiled/rows.rs` | `MsgArgument` | same type in `messagepack/tags.rs`; `validate_argument` | `json_argument`; tag-surface and golden | §12.2 argument |
| Value: `Scalar`, `Array` | `V0_VALUE_TAG_SCALAR = 0`; `V0_VALUE_TAG_ARRAY = 1` | `Value` in `value.rs` | `MsgValue` | same type in `messagepack/tags.rs` | `json_value`; value-tag fixture and golden | §12.2 `Value` |
| Scalar value: `String`, `Integer`, `Float`, `Boolean` | `V0_SCALAR_TAG_STRING = 0`; `V0_SCALAR_TAG_INTEGER = 1`; `V0_SCALAR_TAG_FLOAT = 2`; `V0_SCALAR_TAG_BOOLEAN = 3` | `ScalarValue` in `value.rs` | `MsgScalarValue` and `shared::scalar_value_tag` | structural tag conversion; shared validator finite-float checks | `json_scalar_value`; value-tag fixture and golden | §12.2 `ScalarValue` |
| Availability-reason argument value: `ConditionArg`, `Literal(String)`, `Literal(Int)`, `Literal(Float)`, `Literal(Bool)` | String tags `ConditionArg`, `LiteralString`, `LiteralInt`, `LiteralFloat`, `LiteralBool` (the v0 contract intentionally does not assign numeric constants) | `CompiledAvailabilityReasonArgValue` in `compiled/rows.rs` | `MsgAvailabilityReasonArgValue` | structural `MsgAvailabilityReasonArgValueWrapper`; shared validator finite-float/value checks | `json_availability_reason_arg_binding`; availability-reason and literal-reason fixtures, raw-tag guard, and golden bytes | §12.2 availability-reason argument value |

## Change rule

Before the first tagged release, an intentional v0 correction updates the
model, the shared arity-assertion/tag registry, encoder, decoder/validator,
inspection projection, this matrix, and the focused fixtures together. The
golden snapshot must be reviewed as evidence of the byte change. After the
first tagged release, field additions/removals/reordering, tag changes, or
semantic changes require the `format_version` or
`compiler_compatibility_version` policy in §12.2 instead of silently updating
the v0 snapshot.

Do not make `docs/recite-production-spec.md` depend on the matrix for its
meaning, and do not add a generated serialization abstraction here. The matrix
is a navigation and review aid; the production spec, typed compiled model, and
explicit codec implementations remain the semantic owners.
