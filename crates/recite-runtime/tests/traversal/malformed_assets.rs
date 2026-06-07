use super::*;

#[test]
fn unknown_explicit_block_is_structured_error() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line@ea7507c910e3e3902f7b\n",
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
            "> start_line@b5b9123c411d69b90ff7\n",
            "  Start.\n",
            "-> END\n",
        ),
        "dialogue/first.recitec",
    );
    let second = compile_asset_with_id(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line@6491231b14c3294c494a\n",
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
            "> start_line@787ab6f1aefc7f4b7a72\n",
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
            "> start_line@017500bc41caac76b441\n",
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
            "> start_line@9392649cb4237964b770\n",
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
            "> start_line@c01207cf3dfcf4cb7f0c\n",
            "  Start.\n",
            "-> END\n",
            ":: work\n",
            "> work_line@db8a69cc8b578ab41ec5\n",
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
            "> prompt_line@d9fdc1b49573b701f0ba\n",
            "  What next?\n",
            "  ? ask_work@5457fd1543128230ea43\n",
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
fn missing_availability_reason_reference_is_structured_error() {
    let schema = recite_core::load_schema_manifest_str(
        "fixtures/schema/valid/generated_manifest.json",
        include_str!("../../../../fixtures/schema/valid/generated_manifest.json"),
    )
    .schema
    .expect("valid schema fixture");
    let mut asset = compile_asset_with_schema(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt_line@d70cbd6bb0511ee9a77d\n",
            "  What next?\n",
            "  ? ask_news@094b508364ab0b33473b requires=(trust_gte(hazel, rhea, 3)) reason=innkeeper_trust_hint\n",
            "    Ask for private news.\n",
            "    -> END\n",
        ),
        &schema,
    );
    asset.availability_reasons.clear();
    let context = RecordingContext::default().with("trust_gte", false);
    let mut session = start_scene(&asset, None).expect("starts");

    assert!(matches!(
        next_with_context(&asset, &mut session, &context),
        Err(DialogueError::MalformedCompiledAsset { .. })
    ));
}

#[test]
fn malformed_match_arm_range_is_structured_error() {
    let mut asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line@5402d0a3deb40729e273\n",
            "  Start.\n",
            "-> END\n",
        ),
    );
    asset.statements[0].kind = CompiledStatementKind::Match {
        scrutinee: CompiledConditionCall {
            function: "mood".to_owned(),
            args: Vec::new(),
        },
        arms: MatchArmRange::new(MatchArmIndex::new(99), 1),
    };
    let context = RecordingContext::default().with_enum("mood", "tired");
    let mut session = start_scene(&asset, None).expect("starts");

    assert!(matches!(
        next_with_context(&asset, &mut session, &context),
        Err(DialogueError::MalformedCompiledAsset { .. })
    ));
}

#[test]
fn non_exhaustive_match_is_structured_error() {
    let mut asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            ":match mood()\n",
            "  :case tired\n",
            "    > tired_line@bebaab6b6da928ffd9cd\n",
            "      Tired.\n",
            "-> END\n",
        ),
    );
    let CompiledStatementKind::Match { arms, .. } = asset.statements[0].kind else {
        panic!("expected match statement");
    };
    asset.match_arms = vec![CompiledMatchArm {
        pattern: CompiledMatchPattern::Variant("focused".to_owned()),
        statements: asset.match_arms[arms.start.as_u32() as usize].statements,
        source_map: asset.match_arms[arms.start.as_u32() as usize].source_map,
    }];
    asset.statements[0].kind = CompiledStatementKind::Match {
        scrutinee: CompiledConditionCall {
            function: "mood".to_owned(),
            args: Vec::new(),
        },
        arms: MatchArmRange::new(MatchArmIndex::new(0), 1),
    };
    let context = RecordingContext::default().with_enum("mood", "tired");
    let mut session = start_scene(&asset, None).expect("starts");

    assert!(matches!(
        next_with_context(&asset, &mut session, &context),
        Err(DialogueError::MalformedCompiledAsset { .. })
    ));
}
