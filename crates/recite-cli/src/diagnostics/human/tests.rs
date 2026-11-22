use recite_core::SourceSpan;

pub(in crate::diagnostics) fn excerpt_for_tests(
    span: &SourceSpan,
    text: &str,
) -> (String, usize, usize) {
    super::excerpt(span, text).expect("excerpt")
}
