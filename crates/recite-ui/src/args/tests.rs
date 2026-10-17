use super::*;
use std::borrow::Cow;

#[test]
fn fluent_string_arguments_borrow_the_original_utf8_bytes() {
    let argument = UiArg::from("Un très long paragraphe — 😀".repeat(10_000));
    let UiArg::String(original) = &argument else {
        panic!("string argument")
    };
    let FluentValue::String(Cow::Borrowed(value)) = argument.fluent_value() else {
        panic!("formatting copied the argument buffer")
    };
    assert_eq!(value, original);
    assert_eq!(value.as_ptr(), original.as_ptr());
}
