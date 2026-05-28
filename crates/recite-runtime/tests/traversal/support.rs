use super::*;

#[derive(Debug, Default)]
pub(super) struct RecordingContext {
    results: BTreeMap<String, bool>,
    enum_results: BTreeMap<String, String>,
    failures: BTreeMap<String, String>,
    calls: RefCell<Vec<RecordedCall>>,
}

impl RecordingContext {
    pub(super) fn with(mut self, function: &str, result: bool) -> Self {
        self.results.insert(function.to_owned(), result);
        self
    }

    pub(super) fn with_enum(mut self, function: &str, result: &str) -> Self {
        self.enum_results
            .insert(function.to_owned(), result.to_owned());
        self
    }

    pub(super) fn failing(mut self, function: &str, reason: &str) -> Self {
        self.failures.insert(function.to_owned(), reason.to_owned());
        self
    }

    pub(super) fn calls(&self) -> Vec<RecordedCall> {
        self.calls.borrow().clone()
    }
}

impl recite_runtime::DialogueContext for RecordingContext {
    fn evaluate_condition(
        &self,
        query: ConditionQuery<'_>,
    ) -> Result<recite_runtime::ConditionValue, ConditionEvaluationError> {
        let function = query.function().to_owned();
        let arguments = query
            .arguments()
            .into_iter()
            .map(RecordedArgument::from)
            .collect();
        self.calls.borrow_mut().push(RecordedCall {
            function: function.clone(),
            arguments,
        });

        if let Some(reason) = self.failures.get(&function) {
            return Err(ConditionEvaluationError::new(reason.clone()));
        }

        if let Some(result) = self.results.get(&function) {
            return Ok(recite_runtime::ConditionValue::Bool(*result));
        }
        if let Some(result) = self.enum_results.get(&function) {
            return Ok(recite_runtime::ConditionValue::EnumVariant(result.clone()));
        }

        Err(ConditionEvaluationError::new(format!(
            "missing condition `{function}`"
        )))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct RecordedCall {
    pub(super) function: String,
    pub(super) arguments: Vec<RecordedArgument>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum RecordedArgument {
    Identifier(String),
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
}

impl From<ConditionArgument<'_>> for RecordedArgument {
    fn from(argument: ConditionArgument<'_>) -> Self {
        match argument {
            ConditionArgument::Identifier(value) => Self::Identifier(value.to_owned()),
            ConditionArgument::String(value) => Self::String(value.to_owned()),
            ConditionArgument::Integer(value) => Self::Integer(value),
            ConditionArgument::Float(value) => Self::Float(value),
            ConditionArgument::Boolean(value) => Self::Boolean(value),
        }
    }
}

pub(super) fn deeply_nested_condition(depth: usize) -> CompiledConditionExpression {
    let mut expression = CompiledConditionExpression::Call(CompiledConditionCall {
        function: "trusts".to_owned(),
        args: Vec::new(),
    });

    for _ in 0..depth {
        expression = CompiledConditionExpression::Not(Box::new(expression));
    }

    expression
}

pub(super) fn run_to_end(asset: &CompiledDialogue) -> Vec<DialogueEvent> {
    let mut session = start_scene(asset, None).expect("starts");
    let mut events = Vec::new();

    loop {
        let event = next(asset, &mut session).expect("next succeeds");
        let is_end = matches!(event, DialogueEvent::End { .. });
        events.push(event);
        if is_end {
            break;
        }
    }

    events
}

pub(super) fn run_trace<const N: usize>(
    asset: &CompiledDialogue,
    choice_ids: [&str; N],
) -> Vec<DialogueEvent> {
    let mut session = start_scene(asset, None).expect("starts");
    let mut choices = choice_ids.into_iter();
    let mut events = Vec::new();

    loop {
        let event = next(asset, &mut session).expect("next succeeds");
        let is_prompt = matches!(event, DialogueEvent::Prompt { .. });
        let is_end = matches!(event, DialogueEvent::End { .. });
        events.push(event);

        if is_prompt {
            let choice_id = choices.next().expect("choice provided for prompt");
            let event = choose(
                asset,
                &mut session,
                ChoiceId::new(choice_id).expect("valid choice ID"),
            )
            .expect("choice succeeds");
            let is_end = matches!(event, DialogueEvent::End { .. });
            events.push(event);
            if is_end {
                break;
            }
        } else if is_end {
            break;
        }
    }

    events
}

pub(super) fn next_with_context(
    asset: &CompiledDialogue,
    session: &mut recite_runtime::DialogueSession,
    context: &dyn recite_runtime::DialogueContext,
) -> Result<DialogueEvent, DialogueError> {
    runtime_next(asset, session, context)
}

pub(super) fn choose_with_context(
    asset: &CompiledDialogue,
    session: &mut recite_runtime::DialogueSession,
    choice_id: ChoiceId,
    context: &dyn recite_runtime::DialogueContext,
) -> Result<DialogueEvent, DialogueError> {
    runtime_choose(asset, session, choice_id, context)
}
