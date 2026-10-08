use recite_core::{Diagnostic, DiagnosticRecord};
use recite_runtime::{
    DialogueChoice, DialogueEffectArgument, DialogueEffectMode, DialogueEffectRequest,
    DialogueEvent, DialogueLine,
};
use serde::Serialize;

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum BrowserOutput {
    Diagnostics {
        diagnostics: Vec<DiagnosticRecord>,
    },
    Line {
        line: BrowserLine,
    },
    Prompt {
        line: Option<BrowserLine>,
        choices: Vec<BrowserChoice>,
    },
    Effect {
        effect: BrowserEffect,
    },
    End {
        deferred_effects: Vec<BrowserEffect>,
    },
}

impl BrowserOutput {
    pub(super) fn diagnostics(diagnostics: Vec<Diagnostic>) -> Result<Self, String> {
        let diagnostics = diagnostics
            .iter()
            .map(Diagnostic::record)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        Ok(Self::Diagnostics { diagnostics })
    }
}

impl From<DialogueEvent> for BrowserOutput {
    fn from(event: DialogueEvent) -> Self {
        match event {
            DialogueEvent::Line(line) => Self::Line { line: line.into() },
            DialogueEvent::Prompt { line, choices } => Self::Prompt {
                line: line.map(Into::into),
                choices: choices.into_iter().map(Into::into).collect(),
            },
            DialogueEvent::Effect(effect) => Self::Effect {
                effect: effect.into(),
            },
            DialogueEvent::End { deferred_effects } => Self::End {
                deferred_effects: deferred_effects.into_iter().map(Into::into).collect(),
            },
        }
    }
}

#[derive(Serialize)]
pub(super) struct BrowserLine {
    id: String,
    text: String,
    speaker: Option<String>,
}

impl From<DialogueLine> for BrowserLine {
    fn from(line: DialogueLine) -> Self {
        Self {
            id: line.id.as_str().to_owned(),
            text: line.text,
            speaker: line.speaker.map(|speaker| speaker.as_str().to_owned()),
        }
    }
}

#[derive(Serialize)]
pub(super) struct BrowserChoice {
    id: String,
    text: String,
    available: bool,
    reason: Option<String>,
}

impl From<DialogueChoice> for BrowserChoice {
    fn from(choice: DialogueChoice) -> Self {
        Self {
            id: choice.id.as_str().to_owned(),
            text: choice.text,
            available: choice.availability.is_available,
            reason: choice.availability.primary_reason.map(|reason| reason.text),
        }
    }
}

#[derive(Serialize)]
pub(super) struct BrowserEffect {
    id: String,
    function: String,
    mode: &'static str,
    args: Vec<BrowserArgument>,
}

impl From<DialogueEffectRequest> for BrowserEffect {
    fn from(effect: DialogueEffectRequest) -> Self {
        Self {
            id: effect.id.as_str().to_owned(),
            function: effect.function,
            mode: match effect.mode {
                DialogueEffectMode::Deferred => "deferred",
                DialogueEffectMode::Immediate => "immediate",
                DialogueEffectMode::Blocking => "blocking",
            },
            args: effect.args.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
enum BrowserArgument {
    Identifier(String),
    String(String),
    // Preserve the full i64 range across JavaScript's 53-bit integer boundary.
    Integer(String),
    Float(f64),
    Boolean(bool),
}

impl From<DialogueEffectArgument> for BrowserArgument {
    fn from(argument: DialogueEffectArgument) -> Self {
        match argument {
            DialogueEffectArgument::Identifier(value) => Self::Identifier(value),
            DialogueEffectArgument::String(value) => Self::String(value),
            DialogueEffectArgument::Integer(value) => Self::Integer(value.to_string()),
            DialogueEffectArgument::Float(value) => Self::Float(value),
            DialogueEffectArgument::Boolean(value) => Self::Boolean(value),
        }
    }
}

pub(super) fn encode(output: BrowserOutput) -> Result<String, String> {
    serde_json::to_string(&output).map_err(|error| error.to_string())
}
