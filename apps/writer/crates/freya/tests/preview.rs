mod support;
use freya::prelude::*;
use freya_testing::prelude::*;

fn open_record(test: &mut TestingRunner, summary: &str) -> Result<(), Box<dyn std::error::Error>> {
    let caption = test
        .find(|_, e| {
            Label::try_downcast(e)
                .filter(|l| l.text.starts_with("Open trace record ") && l.text.contains(summary))
                .map(|l| l.text.to_string())
        })
        .ok_or_else(|| format!("Missing trace record {summary}"))?;
    support::click(test, &caption)
}

fn has(test: &TestingRunner, text: &str) -> bool {
    test.find(|_, e| Label::try_downcast(e).filter(|l| l.text.as_ref() == text))
        .is_some()
}

#[test]
fn trial_answers_conditions_and_acknowledges_effects_on_its_own_screen()
-> Result<(), Box<dyn std::error::Error>> {
    let source = ":: start default\n:if trusts(player)\n  > @w3xnt5r\n    Yes.\n:else\n  > @rbgkhpf\n    No.\n! blocking overlay(work)\n-> END\n";
    let mut test = TestingRunner::new(
        recite_writer::regression_app,
        Size2D::new(1200., 1800.),
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
    support::click(&mut test, "What happened in this run?")?;
    open_record(&mut test, "Condition result")?;
    assert!(has(&test, "trusts"));
    assert!(has(&test, "true"));
    assert!(has(&test, "player"));
    support::click(&mut test, "Return to trace records")?;
    open_record(&mut test, "Effect requested")?;
    assert!(has(&test, "overlay"));
    assert!(has(&test, "Blocking"));
    support::click(&mut test, "What happened in this run?")?;
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
    support::click(&mut test, "Continue")?;
    support::click(&mut test, "Report failure")?;
    support::click(&mut test, "What happened in this run?")?;
    support::click(&mut test, "Events")?;
    open_record(&mut test, "Condition result")?;
    assert!(has(&test, "trusts"));
    assert!(has(&test, "false"));
    support::click(&mut test, "Return to trace records")?;
    open_record(&mut test, "Effect acknowledged")?;
    assert!(
        test.find(|_, e| Label::try_downcast(e)
            .filter(|l| l.text.contains("Failure reported in the writer preview")))
            .is_some()
    );
    support::click(&mut test, "Return to writing")?;
    Ok(())
}
