use recite_core::{DiagnosticArgumentValue, PluralRuleError, PoDiagnosticKind, PoDocument};

fn catalogue(rule: &str, arms: usize) -> String {
    let translations = (0..arms)
        .map(|arm| format!("msgstr[{arm}] \"arm {arm}\"\n"))
        .collect::<String>();
    format!(
        "msgid \"\"\nmsgstr \"\"\n\"Plural-Forms: {rule}\\n\"\n\nmsgctxt \"11111111111111111111\"\nmsgid \"one\"\nmsgid_plural \"many\"\n{translations}"
    )
}

#[test]
fn catalogue_loading_accepts_valid_plural_rules() {
    for (rule, arms) in [
        ("nplurals=2; plural=(n != 1);", 2),
        ("nplurals=3; plural=(n == 0 ? 0 : n == 1 ? 1 : 2);", 3),
        ("nplurals=2; plural=(n % 10 != 1);", 2),
        ("nplurals=2; plural=(n == 0 ? 0 : n / n);", 2),
        ("nplurals=1; plural=(n == n ? 0 : 2);", 1),
        ("nplurals=2; plural=(n != 0 && n / n);", 2),
        ("nplurals=2; plural=(n == 9223372036854775807 ? 0 : 1);", 2),
        ("nplurals=2; plural=(n > 0 ? n / n : 0);", 2),
        ("nplurals=2; plural=(n >= 1 ? n / n : 0);", 2),
        ("nplurals=2; plural=(0 < n ? n / n : 0);", 2),
        ("nplurals=2; plural=(n > 0 && n / n);", 2),
        ("nplurals=1; plural=(n <= -1 ? n / n : 0);", 1),
        ("nplurals=1; plural=(-1 >= n ? n / n : 0);", 1),
        ("nplurals=1; plural=(n == -1 ? n + 1 : 0);", 1),
        ("nplurals=1; plural=(n == -9223372036854775807 ? 0 : 0);", 1),
    ] {
        let document = PoDocument::parse(catalogue(rule, arms)).expect("valid rule loads");
        assert_eq!(document.entries()[1].plural_translations().len(), arms);
    }
}

#[test]
fn catalogue_loading_rejects_plural_rules_with_out_of_range_arms() {
    let error = PoDocument::parse(catalogue("nplurals=2; plural=2;", 2))
        .expect_err("an out-of-range arm must be rejected while loading");
    assert!(matches!(
        error.kind(),
        PoDiagnosticKind::InvalidPluralRule(PluralRuleError::ArmOutOfRange {
            arm: 2,
            nplurals: 2
        })
    ));
    let presentation = error
        .diagnostic()
        .presentation
        .as_ref()
        .expect("semantic plural diagnostics have a presentation");
    assert_eq!(
        presentation.id().as_str(),
        "diagnostic-validate-044-invalid-plural-rule"
    );
    assert_eq!(
        presentation.arguments().get("detail"),
        Some(&DiagnosticArgumentValue::String(
            "plural expression selected arm 2, but nplurals is 2".to_owned()
        ))
    );
}

#[test]
fn catalogue_loading_rejects_plural_rules_with_arithmetic_faults() {
    for (expression, expected) in [
        ("n / 0", PluralRuleError::DivisionByZero),
        ("n % 0", PluralRuleError::DivisionByZero),
        (
            "n + 9223372036854775807",
            PluralRuleError::ArithmeticOverflow,
        ),
    ] {
        let rule = format!("nplurals=2; plural={expression};");
        let error = PoDocument::parse(catalogue(&rule, 2))
            .expect_err("an arithmetic fault must be rejected while loading");
        assert!(matches!(
            error.kind(),
            PoDiagnosticKind::InvalidPluralRule(reason) if *reason == expected
        ));
    }
}

#[test]
fn catalogue_loading_rejects_out_of_range_arms_on_only_a_guarded_path() {
    let error = PoDocument::parse(catalogue("nplurals=2; plural=(n == 0 ? 0 : 2);", 2))
        .expect_err("a reachable out-of-range arm must be rejected");
    assert!(matches!(
        error.kind(),
        PoDiagnosticKind::InvalidPluralRule(PluralRuleError::ArmOutOfRange {
            arm: 2,
            nplurals: 2
        })
    ));
}

#[test]
fn catalogue_loading_rejects_arithmetic_when_a_relational_guard_keeps_zero_reachable() {
    let error = PoDocument::parse(catalogue("nplurals=2; plural=(n >= 0 ? n / n : 0);", 2))
        .expect_err("a guard that includes zero must not hide division by zero");
    assert!(matches!(
        error.kind(),
        PoDiagnosticKind::InvalidPluralRule(PluralRuleError::DivisionByZero)
    ));
}
