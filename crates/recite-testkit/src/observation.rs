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
/// then takes the policy's choice. At a blocking effect it checks that
/// advancing and choosing are refused, then acknowledges it. After the end
/// it checks that advancing, choosing and acknowledging are refused.
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
            })) => {
                // A pending blocking effect refuses advancing and choosing.
                probe(dialogue, &mut steps, false);
                Action::Acknowledge
            }
            Ok(Observation::Line(_) | Observation::Effect(_)) => Action::Next,
            Ok(Observation::End { .. }) => {
                // An ended session refuses every operation.
                probe(dialogue, &mut steps, true);
                break;
            }
            Err(_) => break,
        };
    }
    steps
}

/// A choice ID no generated program uses.
const PROBE_CHOICE: &str = "zprobe";

/// Records the refusals of advancing, choosing and, when `acknowledging`,
/// acknowledging an effect.
fn probe(dialogue: &mut dyn Dialogue, steps: &mut Vec<Step>, acknowledging: bool) {
    steps.push(Step {
        action: Action::Next,
        outcome: dialogue.next().map(Some),
    });
    steps.push(Step {
        action: Action::Choose(PROBE_CHOICE.to_owned()),
        outcome: dialogue.choose(PROBE_CHOICE).map(Some),
    });
    if acknowledging {
        steps.push(Step {
            action: Action::Acknowledge,
            outcome: dialogue.acknowledge().map(|()| None),
        });
    }
}
