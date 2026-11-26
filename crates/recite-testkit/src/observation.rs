//! What a host observes from a dialogue, independent of the runtime's types,
//! and a driver that plays a dialogue with a fixed choice policy.

use crate::answers::Answers;
use crate::program::EffectMode;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LineObservation {
    pub id: String,
    pub speaker: Option<String>,
    pub text: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChoiceObservation {
    pub id: String,
    pub text: String,
    pub available: bool,
}

/// An effect request. Request IDs are runtime bookkeeping and are compared
/// separately.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectObservation {
    pub mode: EffectMode,
    pub function: String,
    pub args: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Observation {
    Line(LineObservation),
    Prompt {
        line: Option<LineObservation>,
        choices: Vec<ChoiceObservation>,
    },
    Effect(EffectObservation),
    End {
        deferred: Vec<EffectObservation>,
    },
}

impl Observation {
    /// The observation without authored text, for comparing runs of a
    /// program before and after a text edit.
    #[must_use]
    pub fn without_text(&self) -> Self {
        let line = |line: &LineObservation| LineObservation {
            text: String::new(),
            ..line.clone()
        };
        match self {
            Self::Line(observation) => Self::Line(line(observation)),
            Self::Prompt {
                line: text,
                choices,
            } => Self::Prompt {
                line: text.as_ref().map(line),
                choices: choices
                    .iter()
                    .map(|choice| ChoiceObservation {
                        text: String::new(),
                        ..choice.clone()
                    })
                    .collect(),
            },
            other => other.clone(),
        }
    }

    /// Whether a save taken after this observation names it as its landmark.
    #[must_use]
    pub fn is_landmark(&self) -> bool {
        matches!(self, Self::Line(_) | Self::Prompt { .. })
    }
}

/// Why an operation was refused.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Rejection {
    SessionEnded,
    PromptPending,
    EffectPending,
    NoPromptPending,
    InvalidChoice,
    UnavailableChoice,
    NoEffectPending,
    /// Anything else, such as a malformed asset or a failed condition.
    Other(String),
}

pub type Outcome = Result<Observation, Rejection>;

/// A dialogue a host drives: the reference interpreter or the runtime.
pub trait Dialogue {
    fn next(&mut self) -> Outcome;
    fn choose(&mut self, id: &str) -> Outcome;
    fn acknowledge(&mut self) -> Result<(), Rejection>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Action {
    Next,
    Choose(String),
    Acknowledge,
}

/// One driver operation and its result; an acknowledgement observes nothing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Step {
    pub action: Action,
    pub outcome: Result<Option<Observation>, Rejection>,
}

/// Plays `dialogue` until it ends, is refused, offers no available choice, or
/// `max_steps` operations have run. At each prompt the driver first checks
/// that advancing is refused and that an unavailable choice is refused,
/// then takes the policy's choice; it acknowledges every blocking effect.
pub fn drive(dialogue: &mut dyn Dialogue, answers: &Answers, max_steps: usize) -> Vec<Step> {
    let mut steps = Vec::new();
    let mut action = Action::Next;
    while steps.len() < max_steps {
        let outcome = match &action {
            Action::Next => dialogue.next(),
            Action::Choose(id) => dialogue.choose(id),
            Action::Acknowledge => {
                let acknowledged = dialogue.acknowledge();
                steps.push(Step {
                    action,
                    outcome: acknowledged.map(|()| None),
                });
                action = Action::Next;
                continue;
            }
        };
        steps.push(Step {
            action,
            outcome: outcome.clone().map(Some),
        });
        action = match outcome {
            Ok(Observation::Prompt { choices, .. }) => {
                let refused = dialogue.next();
                steps.push(Step {
                    action: Action::Next,
                    outcome: refused.map(Some),
                });
                if let Some(unavailable) = choices.iter().find(|choice| !choice.available) {
                    let refused = dialogue.choose(&unavailable.id);
                    steps.push(Step {
                        action: Action::Choose(unavailable.id.clone()),
                        outcome: refused.map(Some),
                    });
                }
                let available: Vec<&str> = choices
                    .iter()
                    .filter(|choice| choice.available)
                    .map(|choice| choice.id.as_str())
                    .collect();
                match answers.pick(&available) {
                    Some(id) => Action::Choose(id.to_owned()),
                    None => break,
                }
            }
            Ok(Observation::Effect(EffectObservation {
                mode: EffectMode::Blocking,
                ..
            })) => Action::Acknowledge,
            Ok(Observation::Line(_) | Observation::Effect(_)) => Action::Next,
            Ok(Observation::End { .. }) | Err(_) => break,
        };
    }
    steps
}

/// The observations in a driven run, without refusals and acknowledgements.
#[must_use]
pub fn observations(steps: &[Step]) -> Vec<Observation> {
    steps
        .iter()
        .filter_map(|step| step.outcome.clone().ok().flatten())
        .collect()
}
