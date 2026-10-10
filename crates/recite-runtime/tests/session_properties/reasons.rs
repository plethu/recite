use proptest::prelude::*;
use recite_core::ChoiceId;
use recite_runtime::{
    ConditionQuery, ConditionValue, DialogueError, EmptyDialogueContext, choose, next,
    snapshot::{
        DialogueChoiceAvailabilityReasonArgSnapshot,
        DialogueChoiceAvailabilityReasonOriginSnapshot, DialogueChoiceAvailabilityReasonSnapshot,
        DialogueChoiceAvailabilityReasonTreeSnapshot,
        DialogueChoiceAvailabilityReasonValueSnapshot, decode_session_messagepack,
        encode_session_messagepack, restore_session, snapshot_session,
    },
    start_scene,
};

use super::{failure, fixture};

proptest! {
    #[test]
    fn structured_reason_values_and_origins_survive_restore_without_losing_rejection_state(
        text in ".{0,32}",
        integer in any::<i64>(),
        float in -1_000_000f64..1_000_000f64,
        boolean in any::<bool>(),
        all in any::<bool>(),
    ) {
        let asset = fixture(1)?;
        let mut session = start_scene(&asset, None).map_err(failure)?;
        let context = |_: ConditionQuery<'_>| Ok(ConditionValue::Bool(false));
        next(&asset, &mut session, &context).map_err(failure)?;
        let mut snapshot = snapshot_session(&session);
        let values = vec![
            DialogueChoiceAvailabilityReasonValueSnapshot::Identifier(text.clone()),
            DialogueChoiceAvailabilityReasonValueSnapshot::String(text.clone()),
            DialogueChoiceAvailabilityReasonValueSnapshot::Integer(integer),
            DialogueChoiceAvailabilityReasonValueSnapshot::Float(float),
            DialogueChoiceAvailabilityReasonValueSnapshot::Boolean(boolean),
        ];
        let reason = DialogueChoiceAvailabilityReasonSnapshot {
            id: "blocked".to_owned(), source_text: text.clone(), text: text.clone(),
            origin: Some(DialogueChoiceAvailabilityReasonOriginSnapshot::ConditionCall {
                function: "ready".to_owned(), args: values.clone(),
            }),
            args: values.into_iter().enumerate().map(|(index, value)|
                DialogueChoiceAvailabilityReasonArgSnapshot { name: format!("arg_{index}"), value }).collect(),
        };
        let mut primary = reason.clone();
        primary.origin = Some(DialogueChoiceAvailabilityReasonOriginSnapshot::RequirementExpression { source_text: text.clone() });
        let children = vec![
            DialogueChoiceAvailabilityReasonTreeSnapshot::Reason(reason),
            DialogueChoiceAvailabilityReasonTreeSnapshot::RequirementSourceText(text),
        ];
        let pending = snapshot.pending_prompt.as_mut().ok_or_else(|| TestCaseError::fail("pending prompt"))?;
        pending.choices[0].availability.primary_reason = Some(primary);
        pending.choices[0].availability.reason_tree = Some(if all {
            DialogueChoiceAvailabilityReasonTreeSnapshot::All(children)
        } else { DialogueChoiceAvailabilityReasonTreeSnapshot::Any(children) });
        let expected = snapshot.clone();
        session = restore_session(&asset, snapshot).map_err(failure)?;
        prop_assert_eq!(snapshot_session(&session), expected.clone());
        let bytes = encode_session_messagepack(&session).map_err(failure)?;
        let mut decoded = decode_session_messagepack(&asset, &bytes).map_err(failure)?;
        prop_assert_eq!(snapshot_session(&decoded), expected.clone());
        let id = ChoiceId::new(&expected.previous_prompt_choices[0]).map_err(failure)?;
        let result = choose(&asset, &mut decoded, id, &EmptyDialogueContext);
        prop_assert!(matches!(result, Err(DialogueError::UnavailableChoice { .. })), "unavailable choice must be rejected");
        prop_assert_eq!(snapshot_session(&decoded), expected);
    }
}
