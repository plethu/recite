use super::*;

#[test]
fn plural_lines_reject_additional_body_prose() {
    let source = concat!(
        ":: start default\n",
        "> letters@22222222222222222222 bind=(count:int=$remaining)\n",
        "  You have one letter.\n",
        "  | You have {count} letters.\n",
        "  This would be a third source form.\n",
        "-> END\n",
    );
    let source_file = lower("dialogue/plural.recite", source);
    let report = validate_source_files(&[source_file]);
    assert_codes(&report, ["RECITE_VALIDATE046"]);
}

#[test]
fn count_binding_without_a_continuation_is_ordinary_interpolation() {
    let source = concat!(
        ":: start default\n",
        "> letters@22222222222222222222 bind=(count:int=$remaining)\n",
        "  You have {count} letters.\n",
        "-> END\n",
    );
    let source_file = lower("dialogue/interpolation-count.recite", source);
    let report = validate_source_files(&[source_file]);
    assert_codes(&report, []);
}

#[test]
fn plural_forms_may_use_distinct_non_count_bindings() {
    let source = concat!(
        ":: start default\n",
        "> letters@22222222222222222222 bind=(count:int=$remaining) bind=(name:string=$name)\n",
        "  {name} has one letter.\n",
        "  | You have {count} letters.\n",
        "-> END\n",
    );
    let source_file = lower("dialogue/plural.recite", source);
    let report = validate_source_files(&[source_file]);
    assert!(report.is_ok(), "plural forms should validate: {report:?}");
}

#[test]
fn unsupported_interpolation_type_aliases_are_rejected_before_compilation() {
    for value_type in ["integer", "boolean"] {
        let source = format!(
            ":: start default\n> hello_001@8843fd6f53f020a12b31 bind=(name:{value_type}=$display)\n  Hello, {{name}}.\n-> END\n"
        );
        let report = recite_compiler::compile_inputs(
            [recite_compiler::CompileInput::new(
                "dialogue/interpolation.recite",
                source,
            )],
            recite_compiler::CompileOptions::new(
                recite_core::CompilerVersion::new("0.0.1").expect("valid compiler version"),
                recite_core::CompiledAssetId::new("interpolation").expect("valid asset id"),
                recite_core::SourceMapId::new("interpolation-map").expect("valid source map id"),
                recite_core::SchemaFingerprint::NoSchema,
            ),
        )
        .expect("compilation reports source diagnostics");

        assert!(report.asset.is_none(), "{value_type}");
        assert_eq!(report.diagnostics.len(), 1, "{value_type}");
        let diagnostic = &report.diagnostics[0];
        assert_eq!(diagnostic.code.as_str(), "RECITE_PARSE008");
        assert_eq!(diagnostic.span.start.line(), 2);
        assert_eq!(diagnostic.span.start.column(), 45);
        assert_eq!(
            diagnostic.span.end.map(|position| position.column()),
            Some(44 + value_type.chars().count() as u32)
        );
    }
}
