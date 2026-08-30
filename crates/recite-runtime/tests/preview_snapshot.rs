#[path = "support/preview.rs"]
mod preview_support;

use preview_support::asset;
use recite_core::{LocaleId, ScalarValue};
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
    const GOLDEN_HEX: &str = "87a776657273696f6e02a773657373696f6ede0014b7736e617073686f745f666f726d61745f76657273696f6e01a861737365745f6964b86469616c6f6775652f707265766965772e72656369746563b461737365745f666f726d61745f76657273696f6e00d92461737365745f636f6d70696c65725f636f6d7061746962696c6974795f76657273696f6e00b0636f6d70696c65725f76657273696f6ea5302e302e31ad736f757263655f6d61705f6964b46469616c6f6775652f707265766965772e6d6170b2736368656d615f66696e6765727072696e74a96e6f5f736368656d61a7736f75726365739182a470617468b76469616c6f6775652f707265766965772e726563697465ab66696e6765727072696e7482a9616c676f726974686da6626c616b6533a6646967657374dc0020ccebccb53825207c2ecc9b7d2cccd753ccf5791675ccaf71cca62273186f377fccc254cc9bcc821cccfc3aad63757272656e745f626c6f636b01ad63757272656e745f72616e676582a5737461727404a36c656e02ae6e6578745f73746174656d656e7404b2636f6e74696e756174696f6e5f737461636b90ae70656e64696e675f70726f6d7074c0ae70656e64696e675f656666656374c0b770726576696f75735f70726f6d70745f63686f6963657390b773656c65637465645f63686f6963655f686973746f727990b064656665727265645f6566666563747390a66c6f63616c65a566722d4652ad74726163655f636f756e74657200a5656e646564c2ad696e697469616c5f626c6f636ba9616c7465726e617465a66c6f63616c65a566722d4652a776617269616e74a6666f726d616cb16e6578745f636f6e646974696f6e5f696400a5737461746587a861737365745f6964b86469616c6f6775652f707265766965772e72656369746563a5626c6f636ba9616c7465726e617465a66c6f63616c65a566722d4652b073656c65637465645f63686f6963657390b064656665727265645f6566666563747390b0726573746172745f7265717569726564c0a6737461747573a55265616479";
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

    let marker = [0xa7, b'v', b'e', b'r', b's', b'i', b'o', b'n', 2];
    let mut versioned = encoded;
    let marker_start = versioned
        .windows(marker.len())
        .position(|window| window == marker)
        .expect("version marker")
        + marker.len()
        - 1;
    versioned[marker_start] = 1;
    assert!(matches!(
        recite_runtime::PreviewSnapshot::decode(&versioned),
        Err(recite_runtime::PreviewError::UnsupportedSnapshotFormat {
            snapshot_format_version: 1
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

#[test]
fn rich_v2_golden_covers_prompt_effect_restart_and_plural_provenance() {
    let asset = asset(concat!(
        ":: start default\n",
        "! deferred preload(start)\n",
        "> prompt@12345678901234567890 speaker=hazel mood=calm bind=(count:int=$count)\n",
        "  One item.\n",
        "  | {count} items.\n",
        "  ? keep@12345678901234567891 echo=selected_text tone=plain\n",
        "    Keep.\n",
        "    -> END\n",
    ));
    let mut values = InterpolationValues::new();
    values.insert("count".to_owned(), ScalarValue::from(2_i64));
    let mut preview = PreviewSession::new(&asset, None, PreviewOptions::new()).expect("preview");
    let output = preview.step(PreviewInputs::new().with_interpolation_values(&values));
    assert!(output.events().iter().any(|event| matches!(
        event,
        PreviewEvent::DeferredEffectScheduled(effect) if effect.function == "preload"
    )));
    assert!(output.events().iter().any(|event| matches!(
        event,
        PreviewEvent::Prompt(prompt) if prompt.line().is_some_and(|line| line.plural.is_some())
    )));
    let mut replacement = asset.clone();
    replacement.lines[0].source_text.push('!');
    preview.assess_asset(&replacement).expect("assess");
    let encoded = preview
        .snapshot()
        .expect("snapshot")
        .encode()
        .expect("encode");
    let encoded_hex = encoded
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    const GOLDEN_HEX: &str = "87a776657273696f6e02a773657373696f6ede0014b7736e617073686f745f666f726d61745f76657273696f6e01a861737365745f6964b86469616c6f6775652f707265766965772e72656369746563b461737365745f666f726d61745f76657273696f6e00d92461737365745f636f6d70696c65725f636f6d7061746962696c6974795f76657273696f6e00b0636f6d70696c65725f76657273696f6ea5302e302e31ad736f757263655f6d61705f6964b46469616c6f6775652f707265766965772e6d6170b2736368656d615f66696e6765727072696e74a96e6f5f736368656d61a7736f75726365739182a470617468b76469616c6f6775652f707265766965772e726563697465ab66696e6765727072696e7482a9616c676f726974686da6626c616b6533a6646967657374dc00204c072b76393d0403763eccf1ccfbccfbcc97ccd12875ccbacca3cc9e40cccaccc5cce25bccdaccc66accf3cc9accf4ccc7ad63757272656e745f626c6f636b00ad63757272656e745f72616e676582a5737461727400a36c656e02ae6e6578745f73746174656d656e7402b2636f6e74696e756174696f6e5f737461636b90ae70656e64696e675f70726f6d707482a973746174656d656e7401a763686f696365739182a26964b43132333435363738393031323334353637383931ac617661696c6162696c69747983ac69735f617661696c61626c65c3ae7072696d6172795f726561736f6ec0ab726561736f6e5f74726565c0ae70656e64696e675f656666656374c0b770726576696f75735f70726f6d70745f63686f6963657391b43132333435363738393031323334353637383931b773656c65637465645f63686f6963655f686973746f727990b064656665727265645f656666656374739181a26964d9226566666563743a6469616c6f6775652f707265766965772e7265636974653a323a31a66c6f63616c65c0ad74726163655f636f756e74657201a5656e646564c2ad696e697469616c5f626c6f636bc0a66c6f63616c65c0a776617269616e74c0b16e6578745f636f6e646974696f6e5f696400a5737461746587a861737365745f6964b86469616c6f6775652f707265766965772e72656369746563a5626c6f636ba57374617274a66c6f63616c65c0b073656c65637465645f63686f6963657390b064656665727265645f656666656374739185a26964d9226566666563743a6469616c6f6775652f707265766965772e7265636974653a323a31a46d6f6465a84465666572726564a866756e6374696f6ea77072656c6f6164a4617267739181aa4964656e746966696572a57374617274ab736f757263655f7370616e83a466696c65b76469616c6f6775652f707265766965772e726563697465a5737461727482a46c696e6502a6636f6c756d6e01a3656e64c0b0726573746172745f726571756972656484ac6163746976655f6173736574b86469616c6f6775652f707265766965772e72656369746563b17265706c6163656d656e745f6173736574b86469616c6f6775652f707265766965772e72656369746563af6163746976655f7265766973696f6e82a861737365745f6964b86469616c6f6775652f707265766965772e72656369746563b37061796c6f61645f66696e6765727072696e7482a9616c676f726974686da6626c616b6533a6646967657374dc00201dcc9a52cc837349ccb14c4acce07540ccaaccb96d29cce34eccf3ccd5ccca38ccc6cca317cc9dccce60ccf5391c1db47265706c6163656d656e745f7265766973696f6e82a861737365745f6964b86469616c6f6775652f707265766965772e72656369746563b37061796c6f61645f66696e6765727072696e7482a9616c676f726974686da6626c616b6533a6646967657374dc0020ccb844ccb658cc8a3a7b73ccfc76cc81ccc850ccc3ccd9ccd2ccd902623ecccaccfe0eccbb643bccd7ccb3ccfaccaf23ccf8a673746174757381b057616974696e67466f7243686f69636581a670726f6d707485a5626c6f636ba57374617274a46c696e65b43132333435363738393031323334353637383930a763686f6963657391b43132333435363738393031323334353637383931af6c696e655f70726f6a656374696f6e86a26964b43132333435363738393031323334353637383930ab736f757263655f74657874ae7b636f756e747d206974656d732ea474657874a832206974656d732ea7737065616b6572a568617a656ca86d657461646174619185a36b6579a46d6f6f64a576616c756581a65363616c617281a6537472696e67a463616c6dab736f757263655f7370616e83a466696c65b76469616c6f6775652f707265766965772e726563697465a5737461727482a46c696e6503a6636f6c756d6e2da3656e6482a46c696e6503a6636f6c756d6e35a86b65795f7370616ec0aa76616c75655f7370616ec0a6706c7572616c85b473696e67756c61725f736f757263655f74657874a94f6e65206974656d2eb2706c7572616c5f736f757263655f74657874ae7b636f756e747d206974656d732ea5636f756e7402ac73656c65637465645f61726d01aa7265736f6c7574696f6e87a8617474656d70747390ae6d6174636865645f6c6f63616c65c0af6d6174636865645f636f6e74657874c0ab6d6174636865645f6b6579c0ab6d6174636865645f61726dc0b3736f757263655f66616c6c6261636b5f61726d01a76f7574636f6d65b5456e676c697368536f7572636546616c6c6261636bb163686f6963655f70726f6a656374696f6e9186a26964b43132333435363738393031323334353637383931ab736f757263655f74657874a54b6565702ea474657874a54b6565702eac617661696c6162696c69747983ac69735f617661696c61626c65c3ae7072696d6172795f726561736f6ec0ab726561736f6e5f74726565c0a86d657461646174619185a36b6579a4746f6e65a576616c756581a65363616c617281a6537472696e67a5706c61696eab736f757263655f7370616e83a466696c65b76469616c6f6775652f707265766965772e726563697465a5737461727482a46c696e6506a6636f6c756d6e32a3656e6482a46c696e6506a6636f6c756d6e3ba86b65795f7370616ec0aa76616c75655f7370616ec0a46563686fac53656c656374656454657874";
    assert_eq!(encoded_hex, GOLDEN_HEX);
}
