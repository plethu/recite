use super::*;

#[test]
fn choice_selection_prefers_first_visible_available_choice() {
    let choices = vec![
        TuiChoiceRow {
            index: 1,
            id: "locked".to_owned(),
            text: "Locked.".to_owned(),
            is_available: false,
            unavailable_reason: Some("missing key".to_owned()),
            is_visible: true,
        },
        TuiChoiceRow {
            index: 2,
            id: "open".to_owned(),
            text: "Open.".to_owned(),
            is_available: true,
            unavailable_reason: None,
            is_visible: true,
        },
    ];

    assert_eq!(initial_choice_selection(&choices), 1);
}

#[test]
fn choice_navigation_skips_hidden_and_unavailable_choices() {
    let mut prompt = TuiPrompt::Choice {
        line: None,
        choices: vec![
            TuiChoiceRow {
                index: 1,
                id: "first".to_owned(),
                text: "First.".to_owned(),
                is_available: true,
                unavailable_reason: None,
                is_visible: true,
            },
            TuiChoiceRow {
                index: 2,
                id: "locked".to_owned(),
                text: "Locked.".to_owned(),
                is_available: false,
                unavailable_reason: None,
                is_visible: true,
            },
            TuiChoiceRow {
                index: 3,
                id: "hidden".to_owned(),
                text: "Hidden.".to_owned(),
                is_available: true,
                unavailable_reason: None,
                is_visible: false,
            },
            TuiChoiceRow {
                index: 4,
                id: "last".to_owned(),
                text: "Last.".to_owned(),
                is_available: true,
                unavailable_reason: None,
                is_visible: true,
            },
        ],
        selected: 0,
        interaction: TuiInteractionState::new(PromptMode::Insert),
        input: TextBuffer::default(),
    };

    move_choice_selection(&mut prompt, 1);
    assert_eq!(selected_choice_id(&prompt), Some("last"));
    move_choice_selection(&mut prompt, 1);
    assert_eq!(selected_choice_id(&prompt), Some("first"));
}

#[test]
fn help_mode_can_be_closed_without_changing_stored_prompt_mode() {
    let mut prompt = TuiPrompt::Condition {
        query: "trusts(player)".to_owned(),
        selected: true,
        interaction: TuiInteractionState::new(PromptMode::Insert).with_help(true),
    };

    assert_eq!(prompt_mode(&prompt), PromptMode::Help);
    close_help(&mut prompt);

    assert_eq!(prompt_mode(&prompt), PromptMode::Insert);
}

#[test]
fn help_mode_closes_for_choice_condition_effect_and_finished_prompts() {
    let mut prompts = vec![
        TuiPrompt::Choice {
            line: None,
            choices: Vec::new(),
            selected: 0,
            interaction: TuiInteractionState::new(PromptMode::Insert).with_help(true),
            input: TextBuffer::default(),
        },
        TuiPrompt::Condition {
            query: "trusts(player)".to_owned(),
            selected: true,
            interaction: TuiInteractionState::new(PromptMode::Insert).with_help(true),
        },
        TuiPrompt::EnumCondition {
            query: "memory_pressure(hazel, music_shop)".to_owned(),
            interaction: TuiInteractionState::new(PromptMode::Insert).with_help(true),
            input: TextBuffer::default(),
        },
        TuiPrompt::Effect {
            mode: "blocking".to_owned(),
            id: "effect#1".to_owned(),
            function: "grant_item".to_owned(),
            args: "(key)".to_owned(),
            interaction: TuiInteractionState::new(PromptMode::Insert).with_help(true),
            input: TextBuffer::default(),
        },
        TuiPrompt::Finished {
            interaction: TuiInteractionState::new(PromptMode::Finished).with_help(true),
        },
    ];

    for prompt in &mut prompts {
        assert_eq!(prompt_mode(prompt), PromptMode::Help);
        close_help(prompt);
        assert_ne!(prompt_mode(prompt), PromptMode::Help);
    }
}

#[test]
fn condition_selection_moves_and_sets_answer() {
    let mut prompt = TuiPrompt::Condition {
        query: "trusts(player)".to_owned(),
        selected: true,
        interaction: TuiInteractionState::new(PromptMode::Insert),
    };

    assert_eq!(condition_selection(&prompt), Some(true));
    move_condition_selection(&mut prompt);
    assert_eq!(condition_selection(&prompt), Some(false));
    set_condition_selection(&mut prompt, true);
    assert_eq!(condition_selection(&prompt), Some(true));
}

#[test]
fn enum_condition_prompt_uses_text_input_and_not_boolean_selection() {
    let mut prompt = TuiPrompt::EnumCondition {
        query: "memory_pressure(hazel, music_shop)".to_owned(),
        interaction: TuiInteractionState::new(PromptMode::Normal),
        input: TextBuffer::default(),
    };

    assert_eq!(condition_selection(&prompt), None);
    mutate_prompt_input(&mut prompt, TuiIntent::Text('h'));
    mutate_prompt_input(&mut prompt, TuiIntent::Text('i'));

    assert_eq!(prompt_mode(&prompt), PromptMode::Insert);
    assert_eq!(prompt_input(&prompt), "hi");
}

#[test]
fn deferred_queue_toggle_requires_items() {
    let mut state = TuiState::default();

    toggle_deferred_queue(&mut state);
    assert!(!state.deferred_queue_expanded);

    state.deferred_queue.push(TuiDeferredEffectRow {
        id: "effect:flag#2".to_owned(),
        function: "record_flag".to_owned(),
        args: "(mira_helped)".to_owned(),
    });
    toggle_deferred_queue(&mut state);
    assert!(state.deferred_queue_expanded);
    toggle_deferred_queue(&mut state);
    assert!(!state.deferred_queue_expanded);
}
