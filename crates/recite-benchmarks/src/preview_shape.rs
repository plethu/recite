use recite_runtime::{
    DialogueChoice, DialogueEffectRequest, DialogueLine, PreviewEvent, PreviewPrompt, PreviewTrace,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreviewTraceShape {
    pub event_count: usize,
    pub condition_request_count: usize,
    pub condition_result_count: usize,
    pub line_count: usize,
    pub prompt_count: usize,
    pub choice_accepted_count: usize,
    pub choice_selected_count: usize,
    pub effect_count: usize,
    pub immediate_effect_count: usize,
    pub blocking_effect_count: usize,
    pub deferred_effect_count: usize,
    pub end_count: usize,
    /// Logical collection slots visible in the retained trace. This excludes
    /// allocator headers, spare capacity, and scalar/string storage.
    pub nested_slot_count: usize,
    /// Non-empty trace/event collections, not an exact heap allocation count.
    pub non_empty_collection_count: usize,
    pub localized_lookup_count: usize,
    pub plural_line_count: usize,
}

pub(crate) fn trace_shape(trace: &PreviewTrace) -> PreviewTraceShape {
    let mut shape = PreviewTraceShape {
        event_count: trace.events().len(),
        condition_request_count: 0,
        condition_result_count: 0,
        line_count: 0,
        prompt_count: 0,
        choice_accepted_count: 0,
        choice_selected_count: 0,
        effect_count: 0,
        immediate_effect_count: 0,
        blocking_effect_count: 0,
        deferred_effect_count: 0,
        end_count: 0,
        nested_slot_count: 0,
        non_empty_collection_count: usize::from(!trace.events().is_empty()),
        localized_lookup_count: trace.localized_lookups().count(),
        plural_line_count: trace.plural_lines().count(),
    };
    shape.non_empty_collection_count += usize::from(shape.localized_lookup_count > 0);
    shape.non_empty_collection_count += usize::from(shape.plural_line_count > 0);
    for (_, plural) in trace.plural_lines() {
        shape.nested_slot_count += plural.attempts.len();
        shape.non_empty_collection_count += usize::from(!plural.attempts.is_empty());
    }
    for lookup in trace.localized_lookups() {
        shape.nested_slot_count += lookup.attempts.len();
        shape.non_empty_collection_count += usize::from(!lookup.attempts.is_empty());
    }
    for event in trace.events() {
        match event {
            PreviewEvent::ConditionRequested(request) => {
                shape.condition_request_count += 1;
                add_condition_slots(&mut shape, request.query().arguments().len());
                if let Some(prompt) = request.prompt() {
                    add_prompt_identity_slots(&mut shape, prompt.choices().len());
                }
            }
            PreviewEvent::ConditionResult { request, .. } => {
                shape.condition_result_count += 1;
                add_condition_slots(&mut shape, request.query().arguments().len());
                if let Some(prompt) = request.prompt() {
                    add_prompt_identity_slots(&mut shape, prompt.choices().len());
                }
            }
            PreviewEvent::Line(line) => {
                shape.line_count += 1;
                add_line_slots(&mut shape, line);
            }
            PreviewEvent::Prompt(prompt) => {
                shape.prompt_count += 1;
                add_prompt_slots(&mut shape, prompt);
            }
            PreviewEvent::ChoiceAccepted { prompt, .. } => {
                shape.choice_accepted_count += 1;
                add_prompt_identity_slots(&mut shape, prompt.choices().len());
            }
            PreviewEvent::ChoiceSelected { prompt, .. } => {
                shape.choice_selected_count += 1;
                add_prompt_identity_slots(&mut shape, prompt.choices().len());
            }
            PreviewEvent::EffectRequested(effect) => {
                shape.effect_count += 1;
                match effect.mode {
                    recite_runtime::DialogueEffectMode::Immediate => {
                        shape.immediate_effect_count += 1
                    }
                    recite_runtime::DialogueEffectMode::Blocking => {
                        shape.blocking_effect_count += 1
                    }
                    recite_runtime::DialogueEffectMode::Deferred => {}
                }
                add_effect_slots(&mut shape, effect);
            }
            PreviewEvent::DeferredEffectScheduled(effect) => {
                shape.deferred_effect_count += 1;
                add_effect_slots(&mut shape, effect);
            }
            PreviewEvent::End { deferred_effects } => {
                shape.end_count += 1;
                shape.nested_slot_count += deferred_effects.len();
                shape.non_empty_collection_count += usize::from(!deferred_effects.is_empty());
                for effect in deferred_effects {
                    add_effect_slots(&mut shape, effect);
                }
            }
            PreviewEvent::EffectAcknowledged { .. }
            | PreviewEvent::Restarted { .. }
            | PreviewEvent::Restored
            | PreviewEvent::RestartRequired { .. }
            | PreviewEvent::Error(_) => {}
            _ => {}
        }
    }
    shape
}

fn add_condition_slots(shape: &mut PreviewTraceShape, argument_count: usize) {
    shape.nested_slot_count += argument_count;
    shape.non_empty_collection_count += usize::from(argument_count > 0);
}

fn add_prompt_identity_slots(shape: &mut PreviewTraceShape, choice_count: usize) {
    shape.nested_slot_count += choice_count;
    shape.non_empty_collection_count += usize::from(choice_count > 0);
}

fn add_line_slots(shape: &mut PreviewTraceShape, line: &DialogueLine) {
    shape.nested_slot_count += line.metadata.len();
    shape.non_empty_collection_count += usize::from(!line.metadata.is_empty());
    if let Some(plural) = &line.plural {
        shape.nested_slot_count += plural.resolution.attempts.len();
        shape.non_empty_collection_count += usize::from(!plural.resolution.attempts.is_empty());
    }
}

fn add_prompt_slots(shape: &mut PreviewTraceShape, prompt: &PreviewPrompt) {
    add_prompt_identity_slots(shape, prompt.identity().choices().len());
    if let Some(line) = prompt.line() {
        add_line_slots(shape, line);
    }
    shape.nested_slot_count += prompt.choices().len();
    shape.non_empty_collection_count += usize::from(!prompt.choices().is_empty());
    for choice in prompt.choices() {
        add_choice_slots(shape, choice);
    }
}

fn add_choice_slots(shape: &mut PreviewTraceShape, choice: &DialogueChoice) {
    shape.nested_slot_count += choice.metadata.len();
    shape.non_empty_collection_count += usize::from(!choice.metadata.is_empty());
    if let Some(reason) = &choice.availability.primary_reason {
        shape.nested_slot_count += reason.args.len();
        shape.non_empty_collection_count += usize::from(!reason.args.is_empty());
    }
    if let Some(tree) = &choice.availability.reason_tree {
        add_reason_tree_slots(shape, tree);
    }
}

fn add_reason_tree_slots(
    shape: &mut PreviewTraceShape,
    tree: &recite_runtime::ChoiceAvailabilityReasonTree,
) {
    match tree {
        recite_runtime::ChoiceAvailabilityReasonTree::All(children)
        | recite_runtime::ChoiceAvailabilityReasonTree::Any(children) => {
            shape.nested_slot_count += children.len();
            shape.non_empty_collection_count += usize::from(!children.is_empty());
            for child in children {
                add_reason_tree_slots(shape, child);
            }
        }
        recite_runtime::ChoiceAvailabilityReasonTree::Reason(reason) => {
            shape.nested_slot_count += reason.args.len();
            shape.non_empty_collection_count += usize::from(!reason.args.is_empty());
        }
        recite_runtime::ChoiceAvailabilityReasonTree::RequirementSourceText(_) => {}
    }
}

fn add_effect_slots(shape: &mut PreviewTraceShape, effect: &DialogueEffectRequest) {
    shape.nested_slot_count += effect.args.len();
    shape.non_empty_collection_count += usize::from(!effect.args.is_empty());
}
