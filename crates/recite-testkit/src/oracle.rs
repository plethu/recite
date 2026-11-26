//! A reference interpreter over the lowered source AST.
//!
//! It follows the source semantics directly, without the compiler's tables,
//! statement ranges or continuation frames, so it is an independent oracle
//! for compiled traversal. It is written for clarity, not speed.

use recite_core::ast::{self, Block, Choice, DivertTarget, MatchPattern, SourceFile, Statement};

use crate::answers::{Answer, Answers};
use crate::observation::{
    ChoiceObservation, Dialogue, EffectObservation, LineObservation, Observation, Outcome,
    Rejection,
};
use crate::program::EffectMode;

mod evaluate;
pub(crate) use evaluate::effect_observation;

/// Traversal gives up after this many statements without an observation.
const MAX_STEPS: usize = 10_000;

/// Lowered source files of a program.
#[derive(Clone, Debug)]
pub struct ReferenceProgram {
    pub(crate) files: Vec<SourceFile>,
}

impl ReferenceProgram {
    /// Lowers each `(path, source)`; any parse diagnostic is an error.
    pub fn lower(sources: &[(String, String)]) -> Result<Self, String> {
        let mut files = Vec::with_capacity(sources.len());
        for (path, source) in sources {
            let lowered = recite_parser::lower_source_file(path, source);
            if !lowered.diagnostics.is_empty() {
                return Err(format!("{path} does not parse: {:?}", lowered.diagnostics));
            }
            files.push(lowered.source_file);
        }
        Ok(Self { files })
    }

    /// Starts at the default block.
    pub fn start<'a>(&'a self, answers: &'a Answers) -> Result<OracleSession<'a>, Rejection> {
        let (file, block) = self
            .files
            .iter()
            .enumerate()
            .find_map(|(file, source)| {
                source
                    .blocks
                    .iter()
                    .find(|block| block.is_default)
                    .map(|block| (file, block))
            })
            .ok_or_else(|| Rejection::Other("no default block".to_owned()))?;
        let mut session = OracleSession {
            program: self,
            answers,
            file,
            speaker: None,
            stack: Vec::new(),
            phase: Phase::Running,
            deferred: Vec::new(),
        };
        session.enter(file, block);
        Ok(session)
    }

    fn resolve(&self, from: usize, target: &DivertTarget) -> Option<(usize, &Block)> {
        let DivertTarget::Block(reference) = target else {
            return None;
        };
        let file = match &reference.file {
            Some(path) => self.files.iter().position(|file| file.path == *path)?,
            None => from,
        };
        self.files[file]
            .blocks
            .iter()
            .find(|block| block.id == reference.block_id)
            .map(|block| (file, block))
    }
}

struct Frame<'a> {
    statements: &'a [Statement],
    next: usize,
}

enum Phase<'a> {
    Running,
    Prompt(Vec<(&'a Choice, bool)>),
    Effect,
    Ended,
}

/// A running reference session.
pub struct OracleSession<'a> {
    program: &'a ReferenceProgram,
    answers: &'a Answers,
    file: usize,
    speaker: Option<String>,
    stack: Vec<Frame<'a>>,
    phase: Phase<'a>,
    deferred: Vec<EffectObservation>,
}

impl<'a> OracleSession<'a> {
    pub(crate) fn at(
        program: &'a ReferenceProgram,
        answers: &'a Answers,
        located: crate::position::Located<'a>,
        deferred: Vec<EffectObservation>,
    ) -> Self {
        Self {
            program,
            answers,
            file: located.file,
            speaker: located
                .block
                .default_speaker
                .as_ref()
                .map(|id| id.as_str().to_owned()),
            stack: located
                .frames
                .into_iter()
                .map(|(statements, next)| Frame { statements, next })
                .collect(),
            phase: Phase::Running,
            deferred,
        }
    }

    fn enter(&mut self, file: usize, block: &'a Block) {
        self.file = file;
        self.speaker = block
            .default_speaker
            .as_ref()
            .map(|id| id.as_str().to_owned());
        self.stack = vec![Frame {
            statements: &block.statements,
            next: 0,
        }];
    }

    fn end(&mut self) -> Outcome {
        self.phase = Phase::Ended;
        self.stack.clear();
        Ok(Observation::End {
            deferred: self.deferred.clone(),
        })
    }

    fn divert(&mut self, target: &'a DivertTarget) -> Result<(), Rejection> {
        let (file, block) = self
            .program
            .resolve(self.file, target)
            .ok_or_else(|| Rejection::Other(format!("unknown divert target {target:?}")))?;
        self.enter(file, block);
        Ok(())
    }

    fn advance(&mut self) -> Outcome {
        for _ in 0..MAX_STEPS {
            let Some(frame) = self.stack.last_mut() else {
                return self.end();
            };
            let Some(statement) = frame.statements.get(frame.next) else {
                self.stack.pop();
                continue;
            };
            let statements = frame.statements;
            let index = frame.next;
            frame.next += 1;
            match statement {
                Statement::Comment(_) => {}
                Statement::Line(line) if line.statements.is_empty() => {
                    return Ok(Observation::Line(self.line(line)?));
                }
                Statement::Line(line) => {
                    let choices = line
                        .statements
                        .iter()
                        .filter_map(|statement| match statement {
                            Statement::Choice(choice) => Some(choice),
                            _ => None,
                        });
                    let line = self.line(line)?;
                    return self.prompt(Some(line), choices.collect());
                }
                Statement::Choice(_) => {
                    // Adjacent choices, with comments between them, form one prompt.
                    let mut choices = Vec::new();
                    let mut end = index;
                    while let Some(statement) = statements.get(end) {
                        match statement {
                            Statement::Choice(choice) => choices.push(choice),
                            Statement::Comment(_) => {}
                            _ => break,
                        }
                        end += 1;
                    }
                    if let Some(frame) = self.stack.last_mut() {
                        frame.next = end;
                    }
                    return self.prompt(None, choices);
                }
                Statement::Divert(divert) => match &divert.target {
                    DivertTarget::End => return self.end(),
                    target => self.divert(target)?,
                },
                Statement::If(branch) => {
                    let body = if self.condition(&branch.condition)? {
                        &branch.then_statements
                    } else {
                        &branch.else_statements
                    };
                    self.stack.push(Frame {
                        statements: body,
                        next: 0,
                    });
                }
                Statement::Match(branch) => {
                    let Answer::Variant(variant) = self.call(&branch.scrutinee)? else {
                        return Err(Rejection::Other("scrutinee returned a boolean".to_owned()));
                    };
                    let arm = branch
                        .arms
                        .iter()
                        .find(|arm| arm.pattern == MatchPattern::Variant(variant.to_owned()))
                        .or_else(|| {
                            branch
                                .arms
                                .iter()
                                .find(|arm| arm.pattern == MatchPattern::Wildcard)
                        })
                        .ok_or_else(|| Rejection::Other(format!("no arm for {variant}")))?;
                    self.stack.push(Frame {
                        statements: &arm.statements,
                        next: 0,
                    });
                }
                Statement::Effect(effect) => {
                    let observation = effect_observation(effect);
                    match observation.mode {
                        EffectMode::Deferred => self.deferred.push(observation),
                        EffectMode::Immediate => return Ok(Observation::Effect(observation)),
                        EffectMode::Blocking => {
                            self.phase = Phase::Effect;
                            return Ok(Observation::Effect(observation));
                        }
                    }
                }
            }
        }
        Err(Rejection::Other("traversal step limit".to_owned()))
    }

    fn prompt(&mut self, line: Option<LineObservation>, choices: Vec<&'a Choice>) -> Outcome {
        let mut pending = Vec::with_capacity(choices.len());
        let mut observed = Vec::with_capacity(choices.len());
        for choice in choices {
            let available = match &choice.availability_requirement {
                Some(requirement) => self.condition(&requirement.condition)?,
                None => true,
            };
            observed.push(ChoiceObservation {
                id: choice
                    .id
                    .as_ref()
                    .map(|id| id.as_str().to_owned())
                    .unwrap_or_default(),
                text: choice.source_text.text.clone(),
                available,
            });
            pending.push((choice, available));
        }
        self.phase = Phase::Prompt(pending);
        Ok(Observation::Prompt {
            line,
            choices: observed,
        })
    }

    fn line(&self, line: &ast::Line) -> Result<LineObservation, Rejection> {
        let mut text = line.source_text.text.clone();
        for binding in &line.interpolation_bindings {
            if binding.value.trim_start_matches('$') != "count" {
                return Err(Rejection::Other(format!("no value for {}", binding.value)));
            }
            text = text.replace(
                &format!("{{{}}}", binding.name),
                &self.answers.count.to_string(),
            );
        }
        Ok(LineObservation {
            id: line
                .id
                .as_ref()
                .map(|id| id.as_str().to_owned())
                .unwrap_or_default(),
            speaker: line
                .speaker
                .as_ref()
                .map(|speaker| speaker.as_str().to_owned())
                .or_else(|| self.speaker.clone()),
            text,
        })
    }
}

impl Dialogue for OracleSession<'_> {
    fn next(&mut self) -> Outcome {
        match self.phase {
            Phase::Running => self.advance(),
            Phase::Prompt(_) => Err(Rejection::PromptPending),
            Phase::Effect => Err(Rejection::EffectPending),
            Phase::Ended => Err(Rejection::SessionEnded),
        }
    }

    fn choose(&mut self, id: &str) -> Outcome {
        let target = match &self.phase {
            Phase::Prompt(choices) => {
                let Some((choice, available)) = choices.iter().find(|(choice, _)| {
                    choice
                        .id
                        .as_ref()
                        .is_some_and(|choice| choice.as_str() == id)
                }) else {
                    return Err(Rejection::InvalidChoice);
                };
                if !available {
                    return Err(Rejection::UnavailableChoice);
                }
                choice.target.as_ref().map(|target| &target.target)
            }
            Phase::Effect => return Err(Rejection::EffectPending),
            Phase::Running | Phase::Ended => return Err(Rejection::NoPromptPending),
        };
        self.phase = Phase::Running;
        match target {
            Some(DivertTarget::End) | None => self.end(),
            Some(target) => {
                self.divert(target)?;
                self.advance()
            }
        }
    }

    fn acknowledge(&mut self) -> Result<(), Rejection> {
        if matches!(self.phase, Phase::Effect) {
            self.phase = Phase::Running;
            Ok(())
        } else {
            Err(Rejection::NoEffectPending)
        }
    }
}
