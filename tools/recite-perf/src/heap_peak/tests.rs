use super::*;
#[test]
fn parses_unfiltered_peak_and_rejects_incomplete_logs() {
    assert_eq!(
        peak("==9331== At t-gmax: 101,314 bytes in 1,214 blocks\n").unwrap(),
        101314
    );
    for log in [
        "",
        "At t-gmax: 0 bytes in 0 blocks",
        "At t-gmax: bad bytes in 1 blocks",
        "At t-gmax: 1 bytes in 1 blocks\nAt t-gmax: 2 bytes in 2 blocks",
    ] {
        assert!(peak(log).is_err());
    }
}
