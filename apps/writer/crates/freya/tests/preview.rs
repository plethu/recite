mod support;
use freya::prelude::*;
use freya_testing::prelude::*;

#[test]
fn trial_answers_conditions_and_acknowledges_effects_on_its_own_screen()
-> Result<(), Box<dyn std::error::Error>> {
    let source = ":: start default\n:if trusts(player)\n  > yes@12345678901234567890\n    Yes.\n:else\n  > no@12345678901234567891\n    No.\n! blocking overlay(work)\n-> END\n";
    let mut test = TestingRunner::new(
        recite_writer::regression_app,
        Size2D::new(1200., 900.),
        |runner| {
            runner.provide_root_context(move || recite_writer::InitialSource(source.into()));
        },
        1.,
    )
    .0;
    support::open_beat(&mut test)?;
    support::click(&mut test, "Try scene")?;
    support::click(&mut test, "True")?;
    assert!(
        test.find(|_, e| Label::try_downcast(e).filter(|l| l.text.as_ref() == "Yes."))
            .is_some()
    );
    support::click(&mut test, "Continue")?;
    support::click(&mut test, "Acknowledge completed")?;
    support::click(&mut test, "Continue")?;
    assert!(
        test.find(|_, e| Label::try_downcast(e).filter(|l| l.text.as_ref() == "End conversation"))
            .is_some()
    );
    support::click(&mut test, "Restart preview")?;
    support::click(&mut test, "False")?;
    assert!(
        test.find(|_, e| Label::try_downcast(e).filter(|l| l.text.as_ref() == "No."))
            .is_some()
    );
    support::click(&mut test, "Return to writing")?;
    Ok(())
}
