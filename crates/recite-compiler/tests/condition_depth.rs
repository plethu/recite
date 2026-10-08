#![cfg(test)]

use recite_compiler::{
    authoring::{AuthoringKernel, AuthoringRequest, SavedDocument},
    compile::{
        CompileInput, CompileOptions, CompileReport, compile_inputs, compile_inputs_with_schema,
    },
    pot::{PotExtractionReport, extract_pot, extract_pot_with_schema},
    validation::{ProjectCompleteness, ValidationInput, validate_inputs, validate_source_files},
};
use recite_core::{
    Diagnostic, DiagnosticArgumentValue, DocumentKey,
    ast::{ConditionExpression, SourceFile, Statement},
    compiled::{
        CompiledAssetId, CompilerVersion, MAX_COMPILED_CONDITION_DEPTH, SchemaFingerprint,
        SourceMapId,
    },
    schema::{ProjectSchema, load_schema_manifest_str},
};

const PATH: &str = "dialogue/depth.recite";

fn source(expression: &str, choice: bool) -> String {
    if choice {
        format!(
            ":: start default\n? ask@12345678901234567890 requires=({expression})\n  Continue?\n  -> END\n"
        )
    } else {
        format!(
            ":: start default\n:if {expression}\n  > hello@12345678901234567890\n    Hello.\n-> END\n"
        )
    }
}

// Each layer emits Or -> And -> inner: two semantic edges with only
// one syntactic parenthesis, independently of the parser's nesting limit.
fn mixed_expression(depth: usize) -> String {
    let mut expression = "ready()".to_owned();
    for _ in 0..depth / 2 {
        expression = format!("ready() or ready() and ({expression})");
    }
    if depth % 2 == 1 {
        expression = format!("not ({expression})");
    }
    expression
}

fn root_condition(file: &mut SourceFile) -> &mut ConditionExpression {
    match &mut file.blocks[0].statements[0] {
        Statement::If(branch) => &mut branch.condition,
        Statement::Choice(choice) => {
            &mut choice
                .availability_requirement
                .as_mut()
                .expect("requirement")
                .condition
        }
        _ => panic!("fixture begins with a condition"),
    }
}

fn schema() -> ProjectSchema {
    let report = load_schema_manifest_str(
        "schema.json",
        r#"{"schema_version":1,"conditions":{"ready":{"params":[],"returns":"bool"}}}"#,
    );
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    report.schema.expect("valid ready():bool schema")
}

fn options(schema: Option<&ProjectSchema>) -> CompileOptions {
    CompileOptions::new(
        CompilerVersion::new("0.0.1").expect("compiler version"),
        CompiledAssetId::new("dialogue.recitec").expect("asset ID"),
        SourceMapId::new("dialogue.recitec.map").expect("source map ID"),
        schema.map_or(
            SchemaFingerprint::NoSchema,
            ProjectSchema::canonical_fingerprint,
        ),
    )
}

fn diagnostics(source: &str, schema: Option<&ProjectSchema>) -> Vec<Diagnostic> {
    let lowered = recite_parser::parse(PATH, source).lower_source_file();
    assert!(lowered.diagnostics.is_empty(), "{:?}", lowered.diagnostics);
    validate_inputs(
        [ValidationInput::all_complete(&lowered.source_file)],
        schema,
        ProjectCompleteness::Complete,
    )
    .diagnostics
}

fn compile(source: &str, schema: Option<&ProjectSchema>) -> CompileReport {
    let inputs = [CompileInput::new(PATH, source)];
    match schema {
        Some(schema) => compile_inputs_with_schema(inputs, options(Some(schema)), schema),
        None => compile_inputs(inputs, options(None)),
    }
    .expect("condition depth is a source diagnostic, not an encoding failure")
}

fn extract(source: &str, schema: Option<&ProjectSchema>) -> PotExtractionReport {
    let inputs = [CompileInput::new(PATH, source)];
    match schema {
        Some(schema) => extract_pot_with_schema(inputs, schema),
        None => extract_pot(inputs),
    }
}

fn assert_depth_diagnostic(diagnostics: &[Diagnostic]) {
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    let diagnostic = &diagnostics[0];
    assert_eq!(diagnostic.code.as_str(), "RECITE_VALIDATE050");
    assert_eq!(diagnostic.span.file, PATH);
    assert_eq!(diagnostic.span.start.line(), 2);
    assert!(
        diagnostic
            .span
            .end
            .is_some_and(|end| end > diagnostic.span.start)
    );
    let presentation = diagnostic
        .presentation
        .as_ref()
        .expect("typed presentation");
    assert_eq!(presentation.id().as_str(), "diagnostic-validate-050");
    assert_eq!(
        presentation.arguments().get("limit"),
        Some(&DiagnosticArgumentValue::Integer(
            MAX_COMPILED_CONDITION_DEPTH as i64
        ))
    );
    diagnostic.record().expect("recordable diagnostic");
}

#[test]
fn over_limit_conditions_are_source_diagnostics_in_validation_compilation_and_extraction() {
    let schema = schema();
    for schema in [None, Some(&schema)] {
        for choice in [false, true] {
            let expression = mixed_expression(MAX_COMPILED_CONDITION_DEPTH + 1);
            let source = source(&expression, choice);
            let diagnostics = diagnostics(&source, schema);
            assert_depth_diagnostic(&diagnostics);
            // The diagnostic selects the first overlimit call in the actual
            // source, not an output path or the enclosing statement header.
            let header = source.lines().nth(1).expect("condition header");
            let span = &diagnostics[0].span;
            assert_eq!(
                &header[(span.start.column() - 1) as usize
                    ..span.end.expect("range").column() as usize],
                "ready()"
            );
            let compiled = compile(&source, schema);
            assert!(compiled.asset.is_none());
            assert_eq!(compiled.diagnostics, diagnostics);
            let extracted = extract(&source, schema);
            assert!(extracted.catalog.is_none());
            assert_eq!(extracted.diagnostics, diagnostics);
        }
    }
}

#[test]
fn source_boundary_accepts_grouping_and_wide_boolean_groups() {
    let schema = schema();
    for schema in [None, Some(&schema)] {
        let wide = MAX_COMPILED_CONDITION_DEPTH + 12;
        let parentheses = 120;
        let expressions = [
            mixed_expression(MAX_COMPILED_CONDITION_DEPTH),
            format!(
                "{}ready(){}",
                "(".repeat(parentheses),
                ")".repeat(parentheses)
            ),
            vec!["ready()"; wide].join(" and "),
            vec!["ready()"; wide].join(" or "),
        ];
        for choice in [false, true] {
            for expression in &expressions {
                let source = source(expression, choice);
                assert!(diagnostics(&source, schema).is_empty());
                let compiled = compile(&source, schema);
                assert!(compiled.is_ok(), "{:?}", compiled.diagnostics);
            }
        }
    }
}

#[test]
fn native_ast_boolean_depth_is_bounded_and_grouping_is_semantically_transparent() {
    for choice in [false, true] {
        for operator in ["not", "and", "or", "grouped"] {
            for depth in [
                MAX_COMPILED_CONDITION_DEPTH,
                MAX_COMPILED_CONDITION_DEPTH + 1,
                MAX_COMPILED_CONDITION_DEPTH + 12,
            ] {
                let mut lowered =
                    recite_parser::parse(PATH, source("ready()", choice)).lower_source_file();
                assert!(lowered.diagnostics.is_empty());
                let root = root_condition(&mut lowered.source_file);
                let span = root.span().clone();
                let call = || ConditionExpression::call("ready", Vec::new(), span.clone());
                let mut condition = call();
                for _ in 0..depth {
                    condition = match operator {
                        "not" => ConditionExpression::not(condition, span.clone()),
                        "and" => ConditionExpression::and(vec![condition, call()], span.clone()),
                        "or" => ConditionExpression::or(vec![condition, call()], span.clone()),
                        _ => ConditionExpression::grouped(condition, span.clone()),
                    };
                }
                *root = condition;
                let report = validate_source_files(&[lowered.source_file]);
                if operator == "grouped" || depth <= MAX_COMPILED_CONDITION_DEPTH {
                    assert!(report.is_ok(), "{operator} at {depth}: {report:?}");
                } else {
                    assert_depth_diagnostic(&report.diagnostics);
                }
            }
        }
    }
}

#[test]
fn authoring_recomputes_depth_diagnostics_across_condition_edits() {
    let schema = schema();
    for schema in [None, Some(&schema)] {
        for choice in [false, true] {
            let mut kernel = schema
                .map_or_else(
                    || Ok(AuthoringKernel::new()),
                    |schema| AuthoringKernel::with_schema(schema.clone()),
                )
                .expect("valid condition schema");
            for depth in [
                MAX_COMPILED_CONDITION_DEPTH,
                MAX_COMPILED_CONDITION_DEPTH + 1,
                MAX_COMPILED_CONDITION_DEPTH,
            ] {
                let source = source(&mixed_expression(depth), choice);
                kernel
                    .apply(AuthoringRequest::new(
                        kernel.snapshot().generation(),
                        [SavedDocument::new(
                            DocumentKey::new(PATH).expect("key"),
                            &source,
                        )],
                        [],
                    ))
                    .expect("authoring accepts source and reports its diagnostics");
                let actual: Vec<_> = kernel.snapshot().diagnostics().iter().cloned().collect();
                assert_eq!(actual, diagnostics(&source, schema));
                if depth > MAX_COMPILED_CONDITION_DEPTH {
                    assert_depth_diagnostic(&actual);
                } else {
                    assert!(actual.is_empty());
                }
            }
        }
    }
}
