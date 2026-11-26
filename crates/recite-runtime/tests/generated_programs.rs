//! Compiled traversal of generated programs against the reference
//! interpreter in `recite-testkit`, and the save and resume invariants that
//! must hold for every generated program and patch.

use proptest::collection::vec;
use proptest::prelude::*;
use recite_runtime::start_scene;
use recite_testkit::{ReferenceProgram, Rejection, answers, drive, program};

#[path = "generated_programs/impact.rs"]
mod impact;
#[path = "generated_programs/resume.rs"]
mod resume;
#[path = "generated_programs/support.rs"]
mod support;

use support::{MAX_STEPS, RuntimeDialogue, compile, failure};

proptest! {
    /// For any condition answers and choice path, the compiled program
    /// produces the reference interpreter's observations and refusals, and a
    /// validated program never fails at runtime. Saving, encoding and
    /// restoring the session before any operation changes nothing.
    #[test]
    fn compiled_traversal_matches_the_reference_interpreter(
        program in program(),
        answers in answers(),
        checkpoints in vec(any::<bool>(), 1..6),
    ) {
        let asset = compile(&program)?;
        let reference = ReferenceProgram::lower(&program.sources()).map_err(failure)?;
        let mut oracle = reference.start(&answers).map_err(failure)?;
        let expected = drive(&mut oracle, &answers, MAX_STEPS);

        let session = start_scene(&asset, None).map_err(failure)?;
        let mut live = RuntimeDialogue::new(&asset, session.clone(), &answers, Vec::new());
        let actual = drive(&mut live, &answers, MAX_STEPS);
        prop_assert!(
            !actual.iter().any(|step| matches!(step.outcome, Err(Rejection::Other(_)))),
            "a validated program failed at runtime: {actual:#?}"
        );
        prop_assert_eq!(&actual, &expected);

        let mut restored = RuntimeDialogue::new(&asset, session, &answers, checkpoints);
        let resumed = drive(&mut restored, &answers, MAX_STEPS);
        prop_assert_eq!(restored.failure, None);
        prop_assert_eq!(&resumed, &expected);
        // Restoring keeps effect request IDs and every other event field.
        prop_assert_eq!(restored.events, live.events);
    }
}
