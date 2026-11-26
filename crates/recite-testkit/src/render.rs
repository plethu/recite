//! Render a generated program as `.recite` source.

use std::fmt::Write as _;

use crate::program::{
    ArmSpec, ChoiceSpec, Cond, EffectSpec, LineSpec, Program, Scrutinee, Stmt, Target,
};

impl Program {
    /// Each file's path and source, in declaration order.
    #[must_use]
    pub fn sources(&self) -> Vec<(String, String)> {
        self.files
            .iter()
            .enumerate()
            .map(|(file, spec)| {
                let mut out = String::new();
                for (block, block_spec) in spec.blocks.iter().enumerate() {
                    if block > 0 {
                        out.push('\n');
                    }
                    let default = if file == 0 && block == 0 {
                        " default"
                    } else {
                        ""
                    };
                    let _ = writeln!(out, ":: {}{default}", block_spec.id);
                    Renderer {
                        program: self,
                        file,
                        out: &mut out,
                    }
                    .statements(0, &block_spec.body);
                }
                (spec.path.clone(), out)
            })
            .collect()
    }
}

struct Renderer<'a> {
    program: &'a Program,
    file: usize,
    out: &'a mut String,
}

impl Renderer<'_> {
    fn statements(&mut self, indent: usize, statements: &[Stmt]) {
        for statement in statements {
            self.statement(indent, statement);
        }
    }

    fn statement(&mut self, indent: usize, statement: &Stmt) {
        let pad = " ".repeat(indent);
        match statement {
            Stmt::Line(line) => self.line(indent, line),
            Stmt::Choices(choices) => {
                for (index, choice) in choices.iter().enumerate() {
                    if index > 0 && choice.comment_before {
                        let _ = writeln!(self.out, "{pad}# between choices");
                    }
                    self.choice(indent, choice);
                }
            }
            Stmt::Effect(effect) => {
                let _ = writeln!(
                    self.out,
                    "{pad}! {} {}",
                    effect.mode().keyword(),
                    call(*effect)
                );
            }
            Stmt::If {
                condition,
                then,
                otherwise,
            } => {
                let _ = writeln!(self.out, "{pad}:if {}", cond(condition));
                self.statements(indent + 2, then);
                if !otherwise.is_empty() {
                    let _ = writeln!(self.out, "{pad}:else");
                    self.statements(indent + 2, otherwise);
                }
            }
            Stmt::Match { scrutinee, arms } => {
                let _ = writeln!(self.out, "{pad}:match {}", scrutinee_call(*scrutinee));
                for ArmSpec { pattern, body } in arms {
                    let _ = writeln!(self.out, "{pad}  :case {}", pattern.unwrap_or("_"));
                    self.statements(indent + 4, body);
                }
            }
            Stmt::Divert(target) => {
                let target = self.target(*target);
                let _ = writeln!(self.out, "{pad}-> {target}");
            }
            Stmt::Comment => {
                let _ = writeln!(self.out, "{pad}# generated comment");
            }
        }
    }

    fn line(&mut self, indent: usize, line: &LineSpec) {
        let pad = " ".repeat(indent);
        let mut header = String::new();
        if let Some(speaker) = line.speaker {
            let _ = write!(header, "speaker=s{speaker} ");
        }
        if line.count {
            header.push_str("bind=(count:int=$count) ");
        }
        let _ = writeln!(self.out, "{pad}> {header}@{}", line.id);
        let _ = writeln!(self.out, "{pad}  {}", line_text(line));
        for choice in &line.choices {
            self.choice(indent + 2, choice);
        }
    }

    fn choice(&mut self, indent: usize, choice: &ChoiceSpec) {
        let pad = " ".repeat(indent);
        let requires = choice
            .requires
            .as_ref()
            .map(|condition| format!("requires=({}) ", cond(condition)))
            .unwrap_or_default();
        let target = self.target(choice.target);
        let _ = writeln!(self.out, "{pad}? {requires}@{}", choice.id);
        let _ = writeln!(self.out, "{pad}  {}", choice.text);
        let _ = writeln!(self.out, "{pad}  -> {target}");
    }

    fn target(&self, target: Target) -> String {
        match target {
            Target::End => "END".to_owned(),
            Target::Block { file, block } => {
                let spec = &self.program.files[file];
                if file == self.file {
                    spec.blocks[block].id.clone()
                } else {
                    format!("{}::{}", spec.path, spec.blocks[block].id)
                }
            }
        }
    }
}

/// The authored text of a line, with its placeholder when it binds one.
#[must_use]
pub fn line_text(line: &LineSpec) -> String {
    if line.count {
        format!("Count {{count}}, {}", line.text)
    } else {
        line.text.clone()
    }
}

/// The source form of an effect call.
#[must_use]
pub fn call(effect: EffectSpec) -> String {
    match effect.actor() {
        Some(actor) => format!("{}({})", effect.function(), actor.name()),
        None => format!("{}()", effect.function()),
    }
}

/// The source form of a `:match` scrutinee.
#[must_use]
pub fn scrutinee_call(scrutinee: Scrutinee) -> String {
    match scrutinee {
        Scrutinee::Mood => "mood()".to_owned(),
        Scrutinee::Stage(actor) => format!("stage({})", actor.name()),
    }
}

/// The source form of a condition; compound operands are parenthesised.
#[must_use]
pub fn cond(condition: &Cond) -> String {
    fn operand(condition: &Cond) -> String {
        match condition {
            Cond::And(_) | Cond::Or(_) | Cond::Not(_) => format!("({})", cond(condition)),
            Cond::Flag(_) | Cond::Ready => cond(condition),
        }
    }
    match condition {
        Cond::Flag(actor) => format!("flag({})", actor.name()),
        Cond::Ready => "ready()".to_owned(),
        Cond::Not(inner) => format!("not {}", operand(inner)),
        Cond::And(operands) => join(operands, " and ", operand),
        Cond::Or(operands) => join(operands, " or ", operand),
    }
}

fn join(operands: &[Cond], separator: &str, operand: fn(&Cond) -> String) -> String {
    operands
        .iter()
        .map(operand)
        .collect::<Vec<_>>()
        .join(separator)
}
