use super::*;

#[test]
fn increment_u32_len_reports_overflow() {
    let error = increment_u32_len("choices", u32::MAX).expect_err("overflow is reported");

    assert!(matches!(
        error,
        CompileError::TableIndexOverflow {
            table: "choices",
            ..
        }
    ));
}
