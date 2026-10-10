use super::validate_pending_statement_position;
use crate::DialogueError;
use proptest::prelude::*;
use recite_core::compiled::{StatementIndex, StatementRange};

proptest! {
    #[test]
    fn pending_position_matches_wide_integer_interval_model(
        start in prop_oneof![0u32..100, Just(u32::MAX - 1), Just(u32::MAX)],
        length in 0u32..32,
        offset in 0u32..32,
        valid_next in any::<bool>(),
    ) {
        let statement = start.saturating_add(offset);
        let next = statement.saturating_add(u32::from(valid_next));
        let expected_end = u64::from(start) + u64::from(length);
        let expected_next = u64::from(statement) + 1;
        let expected = expected_end <= u64::from(u32::MAX)
            && expected_next <= u64::from(u32::MAX)
            && u64::from(statement) < expected_end
            && u64::from(next) == expected_next;
        let actual = validate_pending_statement_position(
            "prompt", StatementIndex::new(statement),
            StatementRange { start: StatementIndex::new(start), len: length },
            StatementIndex::new(next),
        );
        match actual {
            Ok(()) => prop_assert!(expected),
            Err(DialogueError::InvalidSessionSnapshot { .. }) => prop_assert!(!expected),
            Err(error) => prop_assert!(false, "unexpected error: {error}"),
        }
    }
}

#[test]
fn statement_before_range_and_nonadjacent_next_are_rejected() {
    let range = StatementRange {
        start: StatementIndex::new(10),
        len: 3,
    };
    for (statement, next) in [(9, 10), (13, 14), (10, 10), (10, 12)] {
        assert!(matches!(
            validate_pending_statement_position(
                "effect",
                StatementIndex::new(statement),
                range,
                StatementIndex::new(next),
            ),
            Err(DialogueError::InvalidSessionSnapshot { .. })
        ));
    }
    assert!(
        validate_pending_statement_position(
            "effect",
            StatementIndex::new(12),
            range,
            StatementIndex::new(13),
        )
        .is_ok()
    );
}
