#[path = "support/preview.rs"]
mod preview_support;

use preview_support::asset;
use recite_core::LocaleId;
use recite_core::ScalarValue;
use recite_runtime::{
    ConditionAnswer, ConditionValue, InterpolationValues, PreviewEvent, PreviewInputs,
    PreviewOptions, PreviewSession,
};

#[test]
fn encoded_snapshot_restores_options_block_and_condition_counter() {
    let asset = asset(concat!(
        ":: start default\n",
        "> first@12345678901234567890\n  First.\n",
        ":if trusts(player)\n",
        "  > second@12345678901234567891\n    Second.\n",
        "-> END\n",
        ":: alternate\n",
        "> alternate_line@12345678901234567892\n  Alternate.\n-> END\n",
    ));
    let options = PreviewOptions::new()
        .with_locale(LocaleId::new("fr-FR").expect("locale"))
        .with_variant("formal");
    let source = PreviewSession::new(&asset, Some("alternate"), options.clone()).expect("source");
    let snapshot = source.snapshot().expect("snapshot");
    let encoded = snapshot.encode().expect("encode");
    const GOLDEN_HEX: &str = "87a776657273696f6e01a773657373696f6ede0014b7736e617073686f745f666f726d61745f76657273696f6e01a861737365745f6964b86469616c6f6775652f707265766965772e72656369746563b461737365745f666f726d61745f76657273696f6e00d92461737365745f636f6d70696c65725f636f6d7061746962696c6974795f76657273696f6e00b0636f6d70696c65725f76657273696f6ea5302e302e31ad736f757263655f6d61705f6964b46469616c6f6775652f707265766965772e6d6170b2736368656d615f66696e6765727072696e74a96e6f5f736368656d61a7736f75726365739182a470617468b76469616c6f6775652f707265766965772e726563697465ab66696e6765727072696e7482a9616c676f726974686da6626c616b6533a6646967657374dc0020ccebccb53825207c2ecc9b7d2cccd753ccf5791675ccaf71cca62273186f377fccc254cc9bcc821cccfc3aad63757272656e745f626c6f636b01ad63757272656e745f72616e676582a5737461727404a36c656e02ae6e6578745f73746174656d656e7404b2636f6e74696e756174696f6e5f737461636b90ae70656e64696e675f70726f6d7074c0ae70656e64696e675f656666656374c0b770726576696f75735f70726f6d70745f63686f6963657390b773656c65637465645f63686f6963655f686973746f727990b064656665727265645f6566666563747390a66c6f63616c65a566722d4652ad74726163655f636f756e74657200a5656e646564c2ad696e697469616c5f626c6f636ba9616c7465726e617465a66c6f63616c65a566722d4652a776617269616e74a6666f726d616cb16e6578745f636f6e646974696f6e5f696400a5737461746587a861737365745f6964b86469616c6f6775652f707265766965772e72656369746563a5626c6f636ba9616c7465726e617465a66c6f63616c65a566722d4652b073656c65637465645f63686f6963657390b064656665727265645f6566666563747390b0726573746172745f7265717569726564c0a6737461747573a55265616479";
    assert_eq!(
        encoded
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
        GOLDEN_HEX
    );
    let decoded = recite_runtime::PreviewSnapshot::decode(&encoded).expect("decode");
    let mut restored = PreviewSession::new(&asset, None, PreviewOptions::new()).expect("receiver");
    restored.restore(decoded).expect("restore");
    assert_eq!(
        restored.state().locale().map(LocaleId::as_str),
        Some("fr-FR")
    );
    assert_eq!(restored.trace().variant(), Some("formal"));
    let restarted = restored.dispatch(
        recite_runtime::PreviewCommand::Restart,
        PreviewInputs::new(),
    );
    assert!(matches!(
        restarted.events(),
        [PreviewEvent::Restarted { block: Some(block), .. }] if block.as_str() == "alternate"
    ));

    let mut original = PreviewSession::new(&asset, None, PreviewOptions::new()).expect("original");
    original.step(PreviewInputs::new());
    let saved = original.snapshot().expect("saved");
    let mut continued =
        PreviewSession::new(&asset, None, PreviewOptions::new()).expect("continued");
    continued.restore(saved).expect("restore saved");
    let left = original.step(PreviewInputs::new());
    let right = continued.step(PreviewInputs::new());
    let left_request = match &left.events()[0] {
        PreviewEvent::ConditionRequested(request) => request,
        event => panic!("expected condition request, got {event:?}"),
    };
    let right_request = match &right.events()[0] {
        PreviewEvent::ConditionRequested(request) => request,
        event => panic!("expected condition request, got {event:?}"),
    };
    assert_eq!(left_request.id(), right_request.id());
    let answer = ConditionAnswer::Value(ConditionValue::Bool(true));
    assert_eq!(
        original
            .answer(left_request.id(), answer.clone(), PreviewInputs::new())
            .events(),
        continued
            .answer(right_request.id(), answer, PreviewInputs::new())
            .events()
    );
}

#[test]
fn preview_snapshot_codec_rejects_trailing_unknown_and_unsupported_data() {
    let asset = asset(":: start default\n> line@12345678901234567890\n  Line.\n-> END\n");
    let preview = PreviewSession::new(&asset, None, PreviewOptions::new()).expect("start");
    let encoded = preview
        .snapshot()
        .expect("snapshot")
        .encode()
        .expect("encode");

    let mut trailing = encoded.clone();
    trailing.push(0);
    assert!(matches!(
        recite_runtime::PreviewSnapshot::decode(&trailing),
        Err(recite_runtime::PreviewError::SnapshotDecodeFailed { .. })
    ));

    let mut unknown = encoded.clone();
    unknown[0] = 0x88;
    unknown.extend_from_slice(&[0xa7, b'u', b'n', b'k', b'n', b'o', b'w', b'n', 0xc0]);
    assert!(matches!(
        recite_runtime::PreviewSnapshot::decode(&unknown),
        Err(recite_runtime::PreviewError::SnapshotDecodeFailed { .. })
    ));

    let marker = [0xa7, b'v', b'e', b'r', b's', b'i', b'o', b'n', 1];
    let mut versioned = encoded;
    let marker_start = versioned
        .windows(marker.len())
        .position(|window| window == marker)
        .expect("version marker")
        + marker.len()
        - 1;
    versioned[marker_start] = 9;
    assert!(matches!(
        recite_runtime::PreviewSnapshot::decode(&versioned),
        Err(recite_runtime::PreviewError::UnsupportedSnapshotFormat {
            snapshot_format_version: 9
        })
    ));
}

#[test]
fn plural_prompt_snapshot_restores_selected_source_projection() {
    let asset = asset(concat!(
        ":: start default\n",
        "> prompt@12345678901234567890 bind=(count:int=$count)\n",
        "  One item.\n  | {count} items.\n",
        "  ? keep@12345678901234567891\n    Keep.\n    -> END\n",
    ));
    let mut values = InterpolationValues::new();
    values.insert("count".to_owned(), ScalarValue::from(2_i64));
    let mut source = PreviewSession::new(&asset, None, PreviewOptions::new()).expect("source");
    let prompt = source.step(PreviewInputs::new().with_interpolation_values(&values));
    assert!(
        matches!(prompt.events(), [PreviewEvent::Prompt(prompt)] if prompt.line().is_some_and(|line| line.text == "2 items."))
    );
    let state = source.state().clone();
    let snapshot = source.snapshot().expect("snapshot");
    let encoded = snapshot.encode().expect("encode");
    let decoded = recite_runtime::PreviewSnapshot::decode(&encoded).expect("decode");
    let mut restored = PreviewSession::new(&asset, None, PreviewOptions::new()).expect("receiver");
    restored.restore(decoded).expect("restore");
    assert_eq!(restored.state(), &state);
}
