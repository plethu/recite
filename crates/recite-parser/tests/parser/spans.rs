use super::*;

#[test]
fn parser_retains_definition_and_source_id_spans() {
    let source = concat!(
        ":: tavern_arrival default speaker=innkeeper\n",
        "> prompt@5e0925cd041f2f6df9e2 speaker=innkeeper\n",
        "  What do you need?\n",
        "  ? ask_news@8d398d18dbde0d7303c2\n",
        "    What's the news?\n",
        "    -> local_news\n",
    );
    let lowered = lower(source);
    assert!(lowered.diagnostics.is_empty());
    let block = single_block(&lowered);
    assert_eq!(
        block
            .id_span
            .as_ref()
            .map(|span| (span.start.line(), span.start.column())),
        Some((1, 4))
    );
    let line = line_statement(block, 0);
    assert_eq!(
        line.source_id_span
            .as_ref()
            .map(|span| (span.start.line(), span.start.column())),
        Some((2, 3))
    );
    let choice = nested_choice(line, 0);
    assert_eq!(
        choice
            .source_id_span
            .as_ref()
            .map(|span| (span.start.line(), span.start.column())),
        Some((4, 5))
    );
}

#[test]
fn parser_retains_exact_divert_target_spans() {
    let source = concat!(
        ":: tavern_arrival\n",
        "-> local_news\n",
        "-> dialogue/market.recite::market_intro\n",
        "-> local_news extra\n",
        "-> dialogue/market.recite::\n",
        "> prompt@5e0925cd041f2f6df9e2\n",
        "  Prompt\n",
        "  ? ask@8d398d18dbde0d7303c2\n",
        "    Choice\n",
        "    -> choice_target\n",
    );
    let lowered = lower(source);
    assert_diagnostic_codes(&lowered, ["RECITE_PARSE011", "RECITE_PARSE011"]);
    assert_eq!(lowered.diagnostics[0].span.start.line(), 4);
    assert_eq!(lowered.diagnostics[0].span.start.column(), 15);
    assert_eq!(lowered.diagnostics[1].span.start.line(), 5);
    assert_eq!(lowered.diagnostics[1].span.start.column(), 4);
    let block = single_block(&lowered);

    let Statement::Divert(local) = &block.statements[0] else {
        panic!("expected local divert");
    };
    let DivertTarget::Block(local_reference) = &local.target else {
        panic!("expected local block target");
    };
    assert_eq!(
        local_reference
            .block_id_span
            .as_ref()
            .map(|span| (span.start.line(), span.start.column())),
        Some((2, 4))
    );

    let Statement::Divert(external) = &block.statements[1] else {
        panic!("expected external divert");
    };
    let DivertTarget::Block(external_reference) = &external.target else {
        panic!("expected external block target");
    };
    assert_eq!(
        external_reference
            .file_span
            .as_ref()
            .map(|span| (span.start.line(), span.start.column())),
        Some((3, 4))
    );
    assert_eq!(
        external_reference
            .block_id_span
            .as_ref()
            .map(|span| (span.start.line(), span.start.column())),
        Some((3, 28))
    );

    let choice = nested_choice(line_statement(block, 2), 0);
    let target = choice.target.as_ref().expect("choice target");
    let DivertTarget::Block(reference) = &target.target else {
        panic!("expected choice block target");
    };
    assert_eq!(
        reference
            .block_id_span
            .as_ref()
            .map(|span| (span.start.line(), span.start.column())),
        Some((10, 8))
    );
}
