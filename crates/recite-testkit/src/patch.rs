//! Small patches to a generated program, of the kinds a game patch makes to
//! its dialogue between releases.

use proptest::collection::vec;
use proptest::prelude::*;

use crate::program::{BlockSpec, ChoiceSpec, Cond, Program, Stmt, Target};
use crate::strategy::{RAW_FILE, body, cond};

/// One edit. Selectors pick a site modulo the number of candidate sites, so
/// every edit applies to any program and shrinks toward the first site.
#[derive(Clone, Debug)]
pub enum PatchOp {
    EditLineText(usize),
    EditChoiceText(usize),
    InsertLine { list: usize, position: usize },
    RemoveStatement { list: usize, position: usize },
    SwapStatements { list: usize, position: usize },
    ChangeEffect(usize),
    ChangeCondition(usize, Cond),
    WrapInIf { list: usize, position: usize },
    Retarget(usize, usize),
    AddChoice(usize),
    RemoveChoice(usize),
    RenameLine(usize),
    AddBlock(Vec<Stmt>),
    RemoveBlock(usize),
}

/// One or two edits.
pub fn patch() -> impl Strategy<Value = Vec<PatchOp>> {
    let site = 0_usize..64;
    let op = prop_oneof![
        3 => site.clone().prop_map(PatchOp::EditLineText),
        1 => site.clone().prop_map(PatchOp::EditChoiceText),
        2 => (site.clone(), site.clone())
            .prop_map(|(list, position)| PatchOp::InsertLine { list, position }),
        2 => (site.clone(), site.clone())
            .prop_map(|(list, position)| PatchOp::RemoveStatement { list, position }),
        1 => (site.clone(), site.clone())
            .prop_map(|(list, position)| PatchOp::SwapStatements { list, position }),
        1 => site.clone().prop_map(PatchOp::ChangeEffect),
        1 => (site.clone(), cond()).prop_map(|(site, cond)| PatchOp::ChangeCondition(site, cond)),
        1 => (site.clone(), site.clone())
            .prop_map(|(list, position)| PatchOp::WrapInIf { list, position }),
        1 => (site.clone(), any::<u8>())
            .prop_map(|(site, target)| PatchOp::Retarget(site, usize::from(target))),
        1 => site.clone().prop_map(PatchOp::AddChoice),
        1 => site.clone().prop_map(PatchOp::RemoveChoice),
        1 => site.clone().prop_map(PatchOp::RenameLine),
        1 => body(1).prop_map(PatchOp::AddBlock),
        1 => site.prop_map(PatchOp::RemoveBlock),
    ];
    vec(op, 1..=2)
}

/// Applies `ops` to a copy of `program` and makes the result valid again.
#[must_use]
pub fn apply_patch(program: &Program, ops: &[PatchOp]) -> Program {
    let mut patched = program.clone();
    for op in ops {
        patched.apply(op);
        patched.normalize();
    }
    patched
}

impl Program {
    fn apply(&mut self, op: &PatchOp) {
        match op {
            PatchOp::EditLineText(site) => self.edit_nth(*site, &mut |statement| match statement {
                Stmt::Line(line) => {
                    line.text = format!("{} Revised.", line.text);
                    true
                }
                _ => false,
            }),
            PatchOp::EditChoiceText(site) => {
                self.edit_nth_choice(*site, &mut |choice| {
                    choice.text = format!("{} Again.", choice.text)
                });
            }
            PatchOp::InsertLine { list, position } => {
                let line = Stmt::Line(self.fresh_line());
                self.edit_list(*list, &mut |statements| {
                    let at = position % (statements.len() + 1);
                    statements.insert(at, line.clone());
                });
            }
            PatchOp::RemoveStatement { list, position } => {
                self.edit_list(*list, &mut |statements| {
                    if !statements.is_empty() {
                        statements.remove(position % statements.len());
                    }
                })
            }
            PatchOp::SwapStatements { list, position } => {
                self.edit_list(*list, &mut |statements| {
                    let movable = statements
                        .iter()
                        .filter(|statement| !statement.is_terminal())
                        .count();
                    if movable >= 2 {
                        let at = position % (movable - 1);
                        statements.swap(at, at + 1);
                    }
                })
            }
            PatchOp::ChangeEffect(site) => self.edit_nth(*site, &mut |statement| match statement {
                Stmt::Effect(effect) => {
                    *effect = match *effect {
                        crate::EffectSpec::Note(mode, actor) => {
                            crate::EffectSpec::Note(mode, other(actor))
                        }
                        crate::EffectSpec::Ping => crate::EffectSpec::Note(
                            crate::EffectMode::Immediate,
                            crate::Actor::Player,
                        ),
                        crate::EffectSpec::Grant(actor) => crate::EffectSpec::Grant(other(actor)),
                    };
                    true
                }
                _ => false,
            }),
            PatchOp::ChangeCondition(site, replacement) => {
                self.edit_nth(*site, &mut |statement| match statement {
                    Stmt::If { condition, .. } => {
                        *condition = replacement.clone();
                        true
                    }
                    _ => false,
                });
            }
            PatchOp::WrapInIf { list, position } => self.edit_list(*list, &mut |statements| {
                if !statements.is_empty() {
                    let at = position % statements.len();
                    let wrapped = statements.remove(at);
                    statements.insert(
                        at,
                        Stmt::If {
                            condition: Cond::Ready,
                            then: vec![wrapped],
                            otherwise: Vec::new(),
                        },
                    );
                }
            }),
            PatchOp::Retarget(site, target) => {
                let target = Target::Block {
                    file: RAW_FILE,
                    block: *target,
                };
                self.edit_nth(*site, &mut |statement| match statement {
                    Stmt::Divert(divert) => {
                        *divert = target;
                        true
                    }
                    _ => false,
                });
            }
            PatchOp::AddChoice(site) => {
                let choice = ChoiceSpec {
                    id: self.fresh_id('c'),
                    text: "Added choice.".to_owned(),
                    requires: None,
                    target: Target::End,
                    comment_before: false,
                };
                self.edit_nth_prompt(*site, &mut |choices| choices.push(choice.clone()));
            }
            PatchOp::RemoveChoice(site) => self.edit_nth_prompt(*site, &mut |choices| {
                if choices.len() > 1 {
                    choices.remove(0);
                }
            }),
            PatchOp::RenameLine(site) => {
                let id = self.fresh_id('l');
                self.edit_nth(*site, &mut |statement| match statement {
                    Stmt::Line(line) => {
                        line.id.clone_from(&id);
                        true
                    }
                    _ => false,
                });
            }
            PatchOp::AddBlock(body) => {
                let id = format!("m{}", self.files[0].blocks.len() + 100);
                self.files[0].blocks.push(BlockSpec {
                    id,
                    body: body.clone(),
                });
            }
            PatchOp::RemoveBlock(site) => self.remove_block(*site),
        }
    }

    /// Edits the `site`-th statement (modulo the count) that `edit` accepts.
    fn edit_nth(&mut self, site: usize, edit: &mut dyn FnMut(&mut Stmt) -> bool) {
        let mut probe = self.clone();
        let mut count = 0;
        probe.visit_mut(&mut |statement| count += usize::from(edit(statement)));
        if count == 0 {
            return;
        }
        let chosen = site % count;
        let mut seen = 0;
        self.visit_mut(&mut |statement| {
            let mut copy = statement.clone();
            if edit(&mut copy) {
                if seen == chosen {
                    *statement = copy;
                }
                seen += 1;
            }
        });
    }

    fn edit_nth_prompt(&mut self, site: usize, edit: &mut dyn FnMut(&mut Vec<ChoiceSpec>)) {
        self.edit_nth(site, &mut |statement| match statement {
            Stmt::Line(line) if !line.choices.is_empty() => {
                edit(&mut line.choices);
                true
            }
            Stmt::Choices(choices) => {
                edit(choices);
                true
            }
            _ => false,
        });
    }

    fn edit_nth_choice(&mut self, site: usize, edit: &mut dyn FnMut(&mut ChoiceSpec)) {
        let mut count = 0;
        self.visit(&mut |statement| count += choices(statement).len());
        if count == 0 {
            return;
        }
        let chosen = site % count;
        let mut seen = 0;
        self.visit_mut(&mut |statement| {
            let list = match statement {
                Stmt::Line(line) => &mut line.choices,
                Stmt::Choices(choices) => choices,
                _ => return,
            };
            for choice in list {
                if seen == chosen {
                    edit(choice);
                }
                seen += 1;
            }
        });
    }

    fn edit_list(&mut self, site: usize, edit: &mut dyn FnMut(&mut Vec<Stmt>)) {
        let mut count = 0;
        self.visit_lists_mut(&mut |_| count += 1);
        let chosen = site % count.max(1);
        let mut seen = 0;
        self.visit_lists_mut(&mut |statements| {
            if seen == chosen {
                edit(statements);
            }
            seen += 1;
        });
    }

    /// Removes a block other than the default, sending its references to
    /// `END`.
    fn remove_block(&mut self, site: usize) {
        let positions = self.block_positions();
        if positions.len() < 2 {
            return;
        }
        let (file, block) = positions[1 + site % (positions.len() - 1)];
        self.files[file].blocks.remove(block);
        let remap = |target: &mut Target| {
            if let Target::Block { file: f, block: b } = target {
                if *f == file && *b == block {
                    *target = Target::End;
                } else if *f == file && *b > block {
                    *b -= 1;
                }
            }
        };
        self.visit_mut(&mut |statement| match statement {
            Stmt::Divert(target) => remap(target),
            Stmt::Line(line) => line
                .choices
                .iter_mut()
                .for_each(|choice| remap(&mut choice.target)),
            Stmt::Choices(choices) => choices
                .iter_mut()
                .for_each(|choice| remap(&mut choice.target)),
            _ => {}
        });
        if self.files[file].blocks.is_empty() {
            self.files.remove(file);
        }
    }
}

fn choices(statement: &Stmt) -> &[ChoiceSpec] {
    match statement {
        Stmt::Line(line) => &line.choices,
        Stmt::Choices(choices) => choices,
        _ => &[],
    }
}

fn other(actor: crate::Actor) -> crate::Actor {
    match actor {
        crate::Actor::Player => crate::Actor::Rhea,
        crate::Actor::Rhea => crate::Actor::Player,
    }
}
