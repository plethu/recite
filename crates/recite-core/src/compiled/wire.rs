//! v0 wire arity and tag constant registry.
//!
//! This file is the shared code registry for v0 fixed-array field counts and
//! `[tag, payload]` tag values. The production spec remains the wire
//! authority; three code/projection surfaces must stay in sync with this
//! registry:
//! the compiler encoder (`crates/recite-compiler/src/wire/messagepack*`), the
//! core decoder (`crate::compiled::messagepack`), and the documented field
//! tables in `docs/recite-production-spec.md` §12.2. Any new compiled row,
//! field, tag, or enum variant updates all of them together.
//!
//! Drift fails close to the change: the tag-surface round-trip test
//! (`recite-compiler/tests/asset/tag_surface.rs`) catches one-sided
//! encoder/decoder drift, and the golden wire-byte snapshot
//! (`recite-compiler/tests/asset/wire_golden.rs`) catches mirrored drift away
//! from the documented layout. Per spec §12.2 the v0 shape stays correctable
//! until the first tagged release; after that, changes here require a
//! `format_version` or `compiler_compatibility_version` bump.

pub const V0_COMPILED_DIALOGUE_FIELDS: u8 = 17;
pub const V0_ASSET_HEADER_FIELDS: u8 = 8;
pub const V0_SOURCE_FILE_FIELDS: u8 = 2;
pub const V0_BLOCK_FIELDS: u8 = 6;
pub const V0_STATEMENT_FIELDS: u8 = 2;
pub const V0_MATCH_ARM_FIELDS: u8 = 3;
pub const V0_LINE_FIELDS: u8 = 5;
pub const V0_CHOICE_FIELDS: u8 = 9;
pub const V0_AVAILABILITY_REASON_FIELDS: u8 = 2;
pub const V0_CONDITION_AVAILABILITY_REASON_FIELDS: u8 = 3;
pub const V0_AVAILABILITY_REASON_ARG_BINDING_FIELDS: u8 = 2;
pub const V0_SPEAKER_FIELDS: u8 = 1;
pub const V0_METADATA_ENTRY_FIELDS: u8 = 3;
pub const V0_EFFECT_FIELDS: u8 = 5;
pub const V0_SOURCE_MAP_ENTRY_FIELDS: u8 = 2;
pub const V0_SOURCE_SPAN_FIELDS: u8 = 5;
pub const V0_LOOKUP_ENTRY_FIELDS: u8 = 2;
pub const V0_RANGE_FIELDS: u8 = 2;
pub const V0_FINGERPRINT_FIELDS: u8 = 2;
pub const V0_TAGGED_VALUE_FIELDS: u8 = 2;
pub const V0_CONDITION_CALL_FIELDS: u8 = 2;
// Payloads of the tagged statement variants are fixed tuples too.
pub const V0_PROMPT_STATEMENT_PAYLOAD_FIELDS: u8 = 2;
pub const V0_IF_STATEMENT_PAYLOAD_FIELDS: u8 = 3;
pub const V0_MATCH_STATEMENT_PAYLOAD_FIELDS: u8 = 2;

pub const V0_ASSET_ENCODING_MESSAGEPACK: u8 = 0;
pub const V0_INSPECTION_ENCODING_COMPACT_JSON: u8 = 0;

pub const V0_SCHEMA_FINGERPRINT_TAG_FINGERPRINT: u8 = 0;
pub const V0_SCHEMA_FINGERPRINT_TAG_NO_SCHEMA: u8 = 1;

pub const V0_STATEMENT_TAG_LINE: u8 = 0;
pub const V0_STATEMENT_TAG_PROMPT: u8 = 1;
pub const V0_STATEMENT_TAG_DIVERT: u8 = 2;
pub const V0_STATEMENT_TAG_IF: u8 = 3;
pub const V0_STATEMENT_TAG_MATCH: u8 = 4;
pub const V0_STATEMENT_TAG_EFFECT: u8 = 5;
pub const V0_STATEMENT_TAG_END: u8 = 6;

pub const V0_MATCH_PATTERN_TAG_VARIANT: u8 = 0;
pub const V0_MATCH_PATTERN_TAG_WILDCARD: u8 = 1;

pub const V0_DIVERT_TARGET_TAG_BLOCK: u8 = 0;
pub const V0_DIVERT_TARGET_TAG_END: u8 = 1;

pub const V0_CHOICE_ECHO_TAG_NONE: u8 = 0;
pub const V0_CHOICE_ECHO_TAG_SELECTED_TEXT: u8 = 1;
pub const V0_CHOICE_ECHO_TAG_EXPLICIT_LINE: u8 = 2;

pub const V0_EFFECT_MODE_TAG_DEFERRED: u8 = 0;
pub const V0_EFFECT_MODE_TAG_IMMEDIATE: u8 = 1;
pub const V0_EFFECT_MODE_TAG_BLOCKING: u8 = 2;

pub const V0_CONDITION_TAG_CALL: u8 = 0;
pub const V0_CONDITION_TAG_AND: u8 = 1;
pub const V0_CONDITION_TAG_OR: u8 = 2;
pub const V0_CONDITION_TAG_NOT: u8 = 3;

pub const V0_ARGUMENT_TAG_IDENTIFIER: u8 = 0;
pub const V0_ARGUMENT_TAG_VALUE: u8 = 1;

pub const V0_SCALAR_TAG_STRING: u8 = 0;
pub const V0_SCALAR_TAG_INTEGER: u8 = 1;
pub const V0_SCALAR_TAG_FLOAT: u8 = 2;
pub const V0_SCALAR_TAG_BOOLEAN: u8 = 3;

pub const V0_VALUE_TAG_SCALAR: u8 = 0;
pub const V0_VALUE_TAG_ARRAY: u8 = 1;
