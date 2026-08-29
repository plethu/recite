use recite_core::{InterpolationType, Statement};
use recite_parser::parse;

#[test]
fn grouped_bindings_are_lowered_with_declared_types() {
    let source = concat!(
        ":: start default\n",
        "> hello_001@8843fd6f53f020a12b31 bind=(name:string=$display)\n",
        "  Hello, {name}!\n",
        "-> END\n",
    );
    let lowered = parse("dialogue/interpolation.recite", source).lower_source_file();
    assert!(lowered.diagnostics.is_empty(), "{:?}", lowered.diagnostics);
    let Statement::Line(line) = &lowered.source_file.blocks[0].statements[0] else {
        panic!("expected line");
    };
    assert_eq!(line.interpolation_bindings[0].name, "name");
    assert_eq!(line.interpolation_bindings[0].value, "display");
    assert_eq!(
        line.interpolation_bindings[0].value_type,
        InterpolationType::String
    );
}

#[test]
fn malformed_grouped_binding_is_a_parse_diagnostic() {
    let source = concat!(
        ":: start default\n",
        "> hello_001@8843fd6f53f020a12b31 bind=name=$display\n",
        "  Hello.\n",
        "-> END\n",
    );
    let lowered = parse("dialogue/interpolation.recite", source).lower_source_file();
    assert!(!lowered.diagnostics.is_empty());
}

#[test]
fn unsupported_interpolation_type_aliases_are_rejected_at_the_type_span() {
    for value_type in ["integer", "boolean"] {
        let source = format!(
            ":: start default\n> hello_001@8843fd6f53f020a12b31 bind=(name:{value_type}=$display)\n  Hello.\n-> END\n"
        );
        let lowered = parse("dialogue/interpolation.recite", &source).lower_source_file();

        assert_eq!(lowered.diagnostics.len(), 1, "{value_type}");
        let diagnostic = &lowered.diagnostics[0];
        assert_eq!(diagnostic.code.as_str(), "RECITE_PARSE008");
        assert_eq!(diagnostic.span.start.line(), 2);
        assert_eq!(diagnostic.span.start.column(), 45);
        assert_eq!(
            diagnostic.span.end.map(|position| position.column()),
            Some(44 + value_type.chars().count() as u32)
        );
    }
}

#[test]
fn canonical_interpolation_types_are_lowered_without_aliasing() {
    for (value_type, expected) in [
        ("string", InterpolationType::String),
        ("int", InterpolationType::Integer),
        ("float", InterpolationType::Float),
        ("bool", InterpolationType::Boolean),
    ] {
        let source = format!(
            ":: start default\n> hello_001@8843fd6f53f020a12b31 bind=(value:{value_type}=$provided)\n  Hello, {{value}}.\n-> END\n"
        );
        let lowered = parse("dialogue/interpolation.recite", &source).lower_source_file();

        assert!(lowered.diagnostics.is_empty(), "{value_type}");
        let Statement::Line(line) = &lowered.source_file.blocks[0].statements[0] else {
            panic!("expected line");
        };
        assert_eq!(line.interpolation_bindings[0].value_type, expected);
    }
}

#[test]
fn plural_continuation_is_lowered_as_its_own_source_text() {
    let source = concat!(
        ":: start default\n",
        "> letters_001@8843fd6f53f020a12b31 bind=(count:int=$remaining)\n",
        "  You have one letter.\n",
        "  | You have {count} letters.\n",
        "-> END\n",
    );
    let lowered = parse("dialogue/plural.recite", source).lower_source_file();
    assert!(lowered.diagnostics.is_empty(), "{:?}", lowered.diagnostics);
    let Statement::Line(line) = &lowered.source_file.blocks[0].statements[0] else {
        panic!("expected line");
    };
    assert_eq!(line.source_text.text, "You have one letter.");
    assert_eq!(
        line.plural_source_text
            .as_ref()
            .map(|text| text.text.as_str()),
        Some("You have {count} letters.")
    );
    assert_eq!(
        line.plural_source_text
            .as_ref()
            .map(|text| text.span.start.line()),
        Some(4)
    );
    assert_eq!(
        line.plural_source_text
            .as_ref()
            .map(|text| text.span.start.column()),
        Some(5)
    );
}

#[test]
fn pipe_on_a_choice_or_non_immediate_line_remains_prose() {
    let source = concat!(
        ":: start default\n",
        "> line_001@8843fd6f53f020a12b31\n",
        "  A line.\n",
        "  Another line.\n",
        "  | A literal pipe.\n",
        "  ? choice@bc0a5874483fd8a329fd\n",
        "    | A choice starting with a pipe.\n",
        "    -> END\n",
        "-> END\n",
    );
    let lowered = parse("dialogue/pipe.recite", source).lower_source_file();
    assert!(lowered.diagnostics.is_empty(), "{:?}", lowered.diagnostics);
    let Statement::Line(line) = &lowered.source_file.blocks[0].statements[0] else {
        panic!("expected line");
    };
    assert!(line.plural_source_text.is_none());
    assert_eq!(
        line.source_text.text,
        "A line.\nAnother line.\n| A literal pipe."
    );
}

#[test]
fn plural_body_without_a_singular_form_remains_prose() {
    let source = concat!(
        ":: start default\n",
        "> letters_001@8843fd6f53f020a12b31 bind=(count:int=$remaining)\n",
        "  | You have {count} letters.\n",
        "-> END\n",
    );
    let lowered = parse("dialogue/plural-missing-singular.recite", source).lower_source_file();
    assert!(lowered.diagnostics.is_empty(), "{:?}", lowered.diagnostics);
    let Statement::Line(line) = &lowered.source_file.blocks[0].statements[0] else {
        panic!("expected line");
    };
    assert!(line.plural_source_text.is_none());
    assert_eq!(line.source_text.text, "| You have {count} letters.");
}

#[test]
fn plural_body_with_an_extra_arm_preserves_it_for_validation() {
    let source = concat!(
        ":: start default\n",
        "> letters_001@8843fd6f53f020a12b31 bind=(count:int=$remaining)\n",
        "  You have one letter.\n",
        "  | You have {count} letters.\n",
        "  | You have many letters.\n",
        "-> END\n",
    );
    let lowered = parse("dialogue/plural-extra-arm.recite", source).lower_source_file();
    assert!(lowered.diagnostics.is_empty(), "{:?}", lowered.diagnostics);
    let Statement::Line(line) = &lowered.source_file.blocks[0].statements[0] else {
        panic!("expected line");
    };
    assert_eq!(
        line.plural_source_text
            .as_ref()
            .map(|text| text.text.as_str()),
        Some("You have {count} letters.")
    );
    assert_eq!(
        line.source_text.text,
        "You have one letter.\n| You have many letters."
    );
}
