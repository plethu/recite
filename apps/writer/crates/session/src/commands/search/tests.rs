use super::*;

#[test]
fn repeat_wraps_in_supplied_order_and_retired_targets_restart_at_the_nearest_end() {
    let matches = ["third", "first", "second"];
    for (accepted, forward, expected) in [
        (None, true, "third"),
        (None, false, "second"),
        (Some("third"), true, "first"),
        (Some("third"), false, "second"),
        (Some("first"), true, "second"),
        (Some("first"), false, "third"),
        (Some("second"), true, "third"),
        (Some("second"), false, "first"),
        (Some("retired"), true, "third"),
        (Some("retired"), false, "second"),
    ] {
        assert_eq!(
            repeat_target(&matches, accepted.as_ref(), forward),
            Some(&expected)
        );
    }
    assert_eq!(repeat_target::<&str>([], None, true), None);
    assert_eq!(
        repeat_target([&"only"], Some(&"only"), false),
        Some(&"only")
    );
}
