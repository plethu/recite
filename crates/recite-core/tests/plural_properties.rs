use proptest::prelude::*;
use recite_core::po::{PluralRuleError, evaluate_plural_form, validate_plural_rule};

proptest! {
    #[test]
    fn modular_plural_rules_match_integer_arithmetic_for_all_generated_counts(
        count in 0i64..=i64::MAX,
        modulus in 2i64..12,
        factor in 1i64..20,
        offset in 0i64..100,
    ) {
        let rule = format!("nplurals={modulus}; plural=((n % {modulus}) * {factor} + {offset}) % {modulus};");
        prop_assert_eq!(validate_plural_rule(&rule), Ok(modulus as usize));
        let expected = ((count % modulus) * factor + offset) % modulus;
        prop_assert_eq!(evaluate_plural_form(&rule, count), Ok(expected as usize));
    }

    #[test]
    fn comparison_and_short_circuit_rules_match_boolean_results(
        count in 0i64..=i64::MAX,
        threshold in 0i64..100,
        operation in 0u8..9,
    ) {
        let (expression, expected) = match operation {
            0 => (format!("n == {threshold}"), count == threshold),
            1 => (format!("n != {threshold}"), count != threshold),
            2 => (format!("n <= {threshold}"), count <= threshold),
            3 => (format!("n >= {threshold}"), count >= threshold),
            4 => (format!("n < {threshold}"), count < threshold),
            5 => (format!("n > {threshold}"), count > threshold),
            6 => ("n == 0 ? 0 : n / n".to_owned(), count != 0),
            7 => ("n != 0 && n / n".to_owned(), count != 0),
            _ => ("n == 0 || n / n".to_owned(), true),
        };
        let rule = format!("nplurals=2; plural={expression};");
        prop_assert_eq!(validate_plural_rule(&rule), Ok(2));
        prop_assert_eq!(evaluate_plural_form(&rule, count), Ok(usize::from(expected)));
    }

    #[test]
    fn guarded_unary_and_division_rules_do_not_fault_outside_the_selected_arm(
        count in 0i64..=i64::MAX,
        divisor in 1i64..20,
    ) {
        let rule = format!("nplurals=2; plural=(n == 0 ? !+n : -(-(n / {divisor} % 2)));");
        let expected = if count == 0 { 1 } else { count / divisor % 2 };
        prop_assert_eq!(validate_plural_rule(&rule), Ok(2));
        prop_assert_eq!(evaluate_plural_form(&rule, count), Ok(expected as usize));
    }
}

#[test]
fn arithmetic_failures_have_stable_typed_errors() {
    for expression in [
        "9223372036854775807 + 1",
        "(-9223372036854775807 - 1) - 1",
        "9223372036854775807 * 2",
        "-(-9223372036854775807 - 1)",
        "(-9223372036854775807 - 1) / -1",
        "(-9223372036854775807 - 1) % -1",
    ] {
        let rule = format!("nplurals=2; plural={expression};");
        assert_eq!(
            evaluate_plural_form(&rule, 0),
            Err(PluralRuleError::ArithmeticOverflow),
            "{expression}"
        );
        assert_eq!(
            validate_plural_rule(&rule),
            Err(PluralRuleError::ArithmeticOverflow),
            "{expression}"
        );
    }
    for expression in ["1 / 0", "1 % 0"] {
        let rule = format!("nplurals=2; plural={expression};");
        assert_eq!(
            evaluate_plural_form(&rule, 0),
            Err(PluralRuleError::DivisionByZero)
        );
        assert_eq!(
            validate_plural_rule(&rule),
            Err(PluralRuleError::DivisionByZero)
        );
    }
}

#[test]
fn unreachable_faults_are_skipped_and_subtraction_selects_the_expected_arm() {
    for (expression, expected) in [
        ("0 && 1 / 0", 0),
        ("1 || 1 / 0", 1),
        ("1 ? 1 : 1 / 0", 1),
        ("0 ? 1 / 0 : 1", 1),
        ("2 - 1", 1),
        ("!1", 0),
    ] {
        let rule = format!("nplurals=2; plural={expression};");
        assert_eq!(validate_plural_rule(&rule), Ok(2));
        assert_eq!(evaluate_plural_form(&rule, i64::MAX), Ok(expected));
    }
}
