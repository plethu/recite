use super::*;

#[test]
fn unknown_explicit_block_is_structured_error() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line\n",
            "  Start.\n",
            "-> END\n",
        ),
    );

    assert_eq!(
        start_scene(&asset, Some("missing")),
        Err(DialogueError::UnknownBlock {
            block: "missing".to_owned()
        })
    );
}

#[test]
fn asset_mismatch_is_structured_error() {
    let first = compile_asset_with_id(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line\n",
            "  Start.\n",
            "-> END\n",
        ),
        "dialogue/first.recitec",
    );
    let second = compile_asset_with_id(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line\n",
            "  Start.\n",
            "-> END\n",
        ),
        "dialogue/second.recitec",
    );
    let mut session = start_scene(&first, None).expect("starts");

    assert!(matches!(
        next(&second, &mut session),
        Err(DialogueError::AssetMismatch { .. })
    ));
}

#[test]
fn malformed_default_block_index_is_structured_error() {
    let mut asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line\n",
            "  Start.\n",
            "-> END\n",
        ),
    );
    asset.default_block = BlockIndex::new(99);

    assert!(matches!(
        start_scene(&asset, None),
        Err(DialogueError::MalformedCompiledAsset { .. })
    ));
}

#[test]
fn malformed_line_index_is_structured_error() {
    let mut asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line\n",
            "  Start.\n",
            "-> END\n",
        ),
    );
    asset.statements[0].kind = CompiledStatementKind::Line(LineIndex::new(99));
    let mut session = start_scene(&asset, None).expect("starts");

    assert!(matches!(
        next(&asset, &mut session),
        Err(DialogueError::MalformedCompiledAsset { .. })
    ));
}

#[test]
fn malformed_effect_index_is_structured_error() {
    let mut asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line\n",
            "  Start.\n",
            "-> END\n",
        ),
    );
    asset.statements[0].kind = CompiledStatementKind::Effect(EffectIndex::new(99));
    let mut session = start_scene(&asset, None).expect("starts");

    assert!(matches!(
        next(&asset, &mut session),
        Err(DialogueError::MalformedCompiledAsset { .. })
    ));
}

#[test]
fn mismatched_explicit_block_lookup_entry_is_structured_error() {
    let mut asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line\n",
            "  Start.\n",
            "-> END\n",
            ":: work\n",
            "> work_line\n",
            "  Work.\n",
            "-> END\n",
        ),
    );
    asset.block_lookup = BlockLookupTable::new(vec![
        BlockLookupEntry {
            id: asset.blocks[0].id.clone(),
            index: BlockIndex::new(0),
        },
        BlockLookupEntry {
            id: asset.blocks[1].id.clone(),
            index: BlockIndex::new(0),
        },
    ])
    .expect("lookup entries remain sorted");

    assert!(matches!(
        start_scene(&asset, Some("work")),
        Err(DialogueError::MalformedCompiledAsset { .. })
    ));
}

#[test]
fn prompt_with_empty_choice_range_is_structured_error() {
    let mut asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt_line\n",
            "  What next?\n",
            "  ? ask_work\n",
            "    Ask about work.\n",
            "    -> END\n",
        ),
    );
    let CompiledStatementKind::Prompt { choices, .. } = &mut asset.statements[0].kind else {
        panic!("expected prompt statement");
    };
    *choices = ChoiceRange::new(choices.start, 0);
    let mut session = start_scene(&asset, None).expect("starts");

    assert!(matches!(
        next(&asset, &mut session),
        Err(DialogueError::MalformedCompiledAsset { .. })
    ));
}

#[test]
fn unsupported_match_statement_is_structured_error() {
    let mut asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line\n",
            "  Start.\n",
            "-> END\n",
        ),
    );
    asset.statements[0].kind = CompiledStatementKind::Match {
        scrutinee: CompiledConditionCall {
            function: "mood".to_owned(),
            args: Vec::new(),
        },
        arms: MatchArmRange::new(MatchArmIndex::new(0), 0),
    };
    let mut session = start_scene(&asset, None).expect("starts");

    assert_eq!(
        next(&asset, &mut session),
        Err(DialogueError::UnsupportedStatement {
            kind: UnsupportedStatementKind::Match
        })
    );
}
