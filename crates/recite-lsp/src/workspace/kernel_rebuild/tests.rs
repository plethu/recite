use super::InputText;
use std::sync::Arc;

#[test]
fn input_text_identity_is_only_a_fast_path_for_exact_equality() {
    let text = InputText(Arc::from("Café 🦀\r\n"));
    let independent = InputText(Arc::from(text.0.to_string()));
    assert!(!Arc::ptr_eq(&text.0, &independent.0));
    assert_eq!(text, text.clone());
    assert_eq!(text, independent);
    assert_ne!(text, InputText(Arc::from("Café 🦀\n")));
    assert_ne!(text, InputText(Arc::from("Other")));
}
