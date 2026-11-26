//! Every rule the specification says validation must reject, with the
//! diagnostic that rejects it. Each case is the generated-program schema and
//! a minimal source that breaks exactly one rule.

use recite_compiler::compile::{CompileInput, CompileOptions, compile_inputs_with_schema};
use recite_core::{
    compiled::{CompiledAssetId, CompilerVersion, SourceMapId},
    schema::load_schema_manifest_str,
};
use recite_testkit::{SCHEMA_JSON, SCHEMA_PATH};

/// Compiles `main` (and `side`, when given) against the generated schema and
/// returns the diagnostic codes in report order.
fn codes(main: &str, side: Option<&str>) -> Vec<String> {
    let schema = load_schema_manifest_str(SCHEMA_PATH, SCHEMA_JSON)
        .schema
        .unwrap_or_else(|| panic!("generated schema is valid"));
    let mut inputs = vec![CompileInput::new("dialogue/main.recite", main)];
    if let Some(side) = side {
        inputs.push(CompileInput::new("dialogue/side.recite", side));
    }
    let report = compile_inputs_with_schema(
        inputs,
        CompileOptions::new(
            CompilerVersion::new("0.0.1").unwrap_or_else(|error| panic!("{error}")),
            CompiledAssetId::new("spec.recitec").unwrap_or_else(|error| panic!("{error}")),
            SourceMapId::new("spec.recitec.map").unwrap_or_else(|error| panic!("{error}")),
            schema.canonical_fingerprint(),
        ),
        &schema,
    )
    .unwrap_or_else(|error| panic!("compile does not hard fail: {error}"));
    report
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str().to_owned())
        .collect()
}

/// Wraps `body` in a default block.
fn block(body: &str) -> String {
    format!(":: start default\n{body}")
}

macro_rules! rejects {
    ($($name:ident: $source:expr => [$($code:literal),+];)+) => {
        $(
            #[test]
            fn $name() {
                assert_eq!(codes(&$source, None), [$($code),+]);
            }
        )+
    };
}

#[test]
fn the_valid_baseline_compiles_cleanly() {
    let source = block(concat!(
        "> speaker=s0 bind=(count:int=$count) @l_ok\n",
        "  Count {count}.\n",
        ":if flag(player) and not ready()\n",
        "  ! immediate note(rhea)\n",
        ":match stage(player)\n",
        "  :case early\n",
        "    ! deferred note(player)\n",
        "  :case _\n",
        "    ! blocking grant(player)\n",
        "? requires=(ready()) @c_ok\n",
        "  Go.\n",
        "  -> dialogue/side.recite::other\n",
    ));
    assert_eq!(
        codes(&source, Some(":: other\n> @l_other\n  Other.\n")),
        Vec::<String>::new()
    );
}

// §5.4.2: missing, malformed and duplicate line and choice IDs.
rejects! {
    line_without_id: block(">\n  Hi.\n") => ["RECITE_ID001"];
    choice_without_id: block("?\n  Go.\n  -> END\n") => ["RECITE_ID002"];
    malformed_id: block("> @Bad\n  Hi.\n") => ["RECITE_ID007"];
    duplicate_line_id: block("> @l_a\n  One.\n> @l_a\n  Two.\n") => ["RECITE_ID003"];
    line_and_choice_share_id: block("> @l_a\n  One.\n? @l_a\n  Go.\n  -> END\n")
        => ["RECITE_ID004"];
}

// §5.1 and §5.8: one default block, and no unknown divert targets.
rejects! {
    two_default_blocks: format!("{}:: again default\n> @l_b\n  B.\n", block("> @l_a\n  A.\n"))
        => ["RECITE_VALIDATE006"];
    unknown_block_target: block("-> nowhere\n") => ["RECITE_VALIDATE007"];
    unknown_file_target: block("-> dialogue/missing.recite::start\n") => ["RECITE_VALIDATE007"];
    unknown_choice_target: block("? @c_a\n  Go.\n  -> nowhere\n") => ["RECITE_VALIDATE007"];
    choice_without_target: block("? @c_a\n  Go.\n") => ["RECITE_VALIDATE012"];
}

// §6.3: condition functions and their arguments.
rejects! {
    unknown_condition: block(":if missing()\n  -> END\n") => ["RECITE_VALIDATE034"];
    condition_wrong_arity: block(":if flag()\n  -> END\n") => ["RECITE_VALIDATE035"];
    condition_wrong_argument_type: block(":if flag(3)\n  -> END\n") => ["RECITE_VALIDATE036"];
    condition_invalid_enum_value: block(":if flag(nobody)\n  -> END\n") => ["RECITE_VALIDATE037"];
    enum_condition_in_if: block(":if mood()\n  -> END\n") => ["RECITE_VALIDATE038"];
    enum_condition_in_requires: block("? requires=(mood()) @c_a\n  Go.\n  -> END\n")
        => ["RECITE_VALIDATE038"];
    boolean_scrutinee: block(":match ready()\n  :case _\n    -> END\n") => ["RECITE_VALIDATE038"];
}

// §5.9.1 and §6.3: `:match` arms.
rejects! {
    undeclared_variant: block(":match mood()\n  :case sleepy\n    -> END\n  :case _\n    -> END\n")
        => ["RECITE_VALIDATE054"];
    non_exhaustive_match: block(":match mood()\n  :case calm\n    -> END\n") => ["RECITE_VALIDATE055"];
    duplicate_variant_arm: block(
        ":match mood()\n  :case calm\n    -> END\n  :case calm\n    -> END\n  :case _\n    -> END\n"
    ) => ["RECITE_VALIDATE052"];
    arm_after_wildcard: block(":match mood()\n  :case _\n    -> END\n  :case calm\n    -> END\n")
        => ["RECITE_VALIDATE053"];
    second_wildcard: block(":match mood()\n  :case _\n    -> END\n  :case _\n    -> END\n")
        => ["RECITE_VALIDATE053"];
}

// §7.6: effect functions, arguments and modes.
rejects! {
    unknown_effect: block("! immediate missing()\n") => ["RECITE_VALIDATE017"];
    effect_wrong_arity: block("! immediate note()\n") => ["RECITE_VALIDATE018"];
    effect_wrong_argument_type: block("! immediate note(3)\n") => ["RECITE_VALIDATE019"];
    effect_invalid_enum_value: block("! immediate note(nobody)\n") => ["RECITE_VALIDATE021"];
    effect_unsupported_mode: block("! deferred ping()\n") => ["RECITE_VALIDATE020"];
}

// §5.10 and §5.11: interpolation bindings and plural lines.
rejects! {
    undeclared_placeholder: block("> @l_a\n  Hi {name}.\n") => ["RECITE_VALIDATE045"];
    unused_binding: block("> bind=(count:int=$count) @l_a\n  Hi.\n") => ["RECITE_VALIDATE045"];
    plural_without_count: block("> @l_a\n  One.\n  | Many.\n") => ["RECITE_VALIDATE046"];
}

// §9.2: a translation must keep its source's placeholders.
#[test]
fn translations_with_missing_renamed_or_extra_placeholders() {
    use recite_core::po::{PoDiagnosticKind, PoDocument};

    for (translation, expected) in [
        ("Bonjour.", "missing {name}"),
        ("Bonjour {nom}.", "extra {nom}"),
        ("Bonjour {name} {extra}.", "extra {extra}"),
    ] {
        let catalogue =
            format!("msgctxt \"l_a\"\nmsgid \"Hello {{name}}.\"\nmsgstr \"{translation}\"\n");
        let error = PoDocument::parse(&catalogue)
            .err()
            .unwrap_or_else(|| panic!("`{translation}` must be rejected"));
        assert!(
            matches!(error.kind(), PoDiagnosticKind::PlaceholderMismatch(detail) if detail.contains(expected)),
            "{translation}: {:?}",
            error.kind()
        );
    }
}
