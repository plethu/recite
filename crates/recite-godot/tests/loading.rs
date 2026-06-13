mod support;

use recite_godot::{ReciteDialogueAsset, ReciteDialogueDriver};

use support::{
    assert_deferred_effects, assert_effect, assert_line, assert_prompt_choice_ids, compile_asset,
    compile_bytes, must_ok, output_kinds, temp_asset_path,
};

#[test]
fn loads_messagepack_bytes_and_runs_full_scene_flow() {
    let asset = compile_asset(
        "dialogue/full.recite",
        "dialogue/full.recitec",
        concat!(
            ":: start default\n",
            "> intro@11111111111111111111\n",
            "  Hello.\n",
            "> prompt@11111111111111111112\n",
            "  Choose.\n",
            "  ? work@11111111111111111113\n",
            "    Work.\n",
            "    -> work\n",
            "  ? leave@11111111111111111114\n",
            "    Leave.\n",
            "    -> END\n",
            ":: work\n",
            "> work_line@11111111111111111115\n",
            "  Work waits.\n",
            "! immediate ping(work)\n",
            "! deferred remember(work)\n",
            "-> END\n",
        ),
    );
    let mut driver = ReciteDialogueDriver::new();

    let start_outputs = must_ok(driver.start(&asset, None, Some("en-GB")));
    assert_eq!(output_kinds(&start_outputs), ["line", "prompt"]);
    assert_line(&start_outputs[0], "11111111111111111111", "Hello.");
    assert_prompt_choice_ids(
        &start_outputs[1],
        ["11111111111111111113", "11111111111111111114"],
    );

    let choice_outputs = must_ok(driver.select_choice("11111111111111111113"));
    assert_eq!(output_kinds(&choice_outputs), ["line", "effect", "end"]);
    assert_line(&choice_outputs[0], "11111111111111111115", "Work waits.");
    assert_effect(&choice_outputs[1], "ping", "immediate");
    assert_deferred_effects(&choice_outputs[2], ["remember"]);
}

#[test]
fn loads_messagepack_from_path() {
    let bytes = compile_bytes(
        "dialogue/path.recite",
        "dialogue/path.recitec",
        concat!(
            ":: start default\n",
            "> intro@22222222222222222221\n",
            "  Loaded from disk.\n",
            "-> END\n",
        ),
    );
    let path = temp_asset_path("recite-godot-load-path.recitec");
    if let Err(error) = std::fs::write(&path, &bytes) {
        panic!("failed to write test asset: {error}");
    }

    let asset = match ReciteDialogueAsset::load_from_path(&path) {
        Ok(asset) => asset,
        Err(error) => panic!("asset should load from path: {error}"),
    };
    let _ = std::fs::remove_file(&path);

    let mut driver = ReciteDialogueDriver::new();
    let outputs = must_ok(driver.start(&asset, None, None));
    assert_eq!(output_kinds(&outputs), ["line", "end"]);
    assert_line(&outputs[0], "22222222222222222221", "Loaded from disk.");
}

#[test]
fn malformed_messagepack_uses_contract_error_category() {
    let error = match ReciteDialogueAsset::load_from_bytes(b"not messagepack") {
        Ok(_) => panic!("malformed compiled bytes should fail"),
        Err(error) => error,
    };

    assert_eq!(error.code(), "asset_load_or_decode_error");
}
