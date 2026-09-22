use recite_writer_model::{ConditionValue, Document, EffectAck, Preview};

#[test]
fn pending_condition_can_be_answered_without_recompiling_the_trial()
-> Result<(), Box<dyn std::error::Error>> {
    let document = Document::new(
        ":: start default\n:if trusts(player)\n  > yes@12345678901234567890\n    Yes.\n:else\n  > no@12345678901234567891\n    No.\n-> END\n",
    )?;
    let mut preview = Preview::new(&document)?;
    let page = preview.advance(None)?;
    let request = page.condition.ok_or("condition request")?;
    assert!(
        preview
            .answer(&request, ConditionValue::EnumVariant("wrong".into()))
            .is_err()
    );
    let page = preview.answer(&request, ConditionValue::Bool(true))?;
    assert_eq!(page.text, "Yes.");
    assert!(page.condition.is_none());
    assert!(preview.advance(None)?.ended);
    Ok(())
}

#[test]
fn blocking_effect_waits_for_ack_and_deferred_requests_survive_to_end()
-> Result<(), Box<dyn std::error::Error>> {
    let document = Document::new(
        ":: start default\n! blocking overlay(work)\n! deferred finished(work)\n-> END\n",
    )?;
    let mut preview = Preview::new(&document)?;
    let page = preview.advance(None)?;
    let id = page.waiting_effect.ok_or("blocking request")?;
    preview.acknowledge(id, EffectAck::Completed)?;
    let mut ended = None;
    for _ in 0..4 {
        let page = preview.advance(None)?;
        if page.ended {
            ended = Some(page);
            break;
        }
    }
    let end = ended.ok_or("scene ended")?;
    assert_eq!(end.effects.len(), 1);
    assert_eq!(end.effects[0].function, "finished");
    Ok(())
}
