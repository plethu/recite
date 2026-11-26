//! Make a raw or patched program valid, and visit its statements.

use crate::program::{ChoiceSpec, LineSpec, Program, Stmt, Target};
use crate::strategy::RAW_FILE;

impl Program {
    /// Assigns fresh IDs to new lines and choices, resolves target selectors,
    /// retargets diverts that would not move forward to `END`, drops
    /// statements after a prompt or divert, and gives every statement list a
    /// statement that is not a comment.
    pub fn normalize(&mut self) {
        let positions = self.block_positions();
        for (file, block) in positions.clone() {
            let current = self.global_index(file, block);
            let mut body = std::mem::take(&mut self.files[file].blocks[block].body);
            self.fix_list(&mut body, current, &positions, true);
            self.files[file].blocks[block].body = body;
        }
    }

    fn fix_list(
        &mut self,
        statements: &mut Vec<Stmt>,
        current: usize,
        positions: &[(usize, usize)],
        required: bool,
    ) {
        if let Some(end) = statements.iter().position(Stmt::is_terminal) {
            statements.truncate(end + 1);
        }
        if statements
            .iter()
            .all(|statement| *statement == Stmt::Comment)
        {
            if required {
                statements.insert(0, Stmt::Line(self.fresh_line()));
            } else {
                statements.clear();
            }
        }
        for statement in statements {
            match statement {
                Stmt::Line(line) => {
                    if line.id.is_empty() {
                        line.id = self.fresh_id('l');
                    }
                    for choice in &mut line.choices {
                        self.fix_choice(choice, positions);
                    }
                }
                Stmt::Choices(choices) => {
                    for choice in choices {
                        self.fix_choice(choice, positions);
                    }
                }
                Stmt::Divert(target) => *target = divert_target(*target, current, positions),
                Stmt::If {
                    then, otherwise, ..
                } => {
                    self.fix_list(then, current, positions, true);
                    self.fix_list(otherwise, current, positions, false);
                }
                Stmt::Match { arms, .. } => {
                    for arm in arms {
                        self.fix_list(&mut arm.body, current, positions, true);
                    }
                }
                Stmt::Effect(_) | Stmt::Comment => {}
            }
        }
    }

    fn fix_choice(&mut self, choice: &mut ChoiceSpec, positions: &[(usize, usize)]) {
        if choice.id.is_empty() {
            choice.id = self.fresh_id('c');
        }
        choice.target = match choice.target {
            Target::Block {
                file: RAW_FILE,
                block,
            } => positions
                .get(block % positions.len().max(1))
                .map_or(Target::End, |&(file, block)| Target::Block { file, block }),
            Target::Block { file, block } if !positions.contains(&(file, block)) => Target::End,
            target => target,
        };
    }

    /// A new line with a fresh ID, for patches and empty statement lists.
    pub fn fresh_line(&mut self) -> LineSpec {
        let id = self.fresh_id('l');
        LineSpec {
            text: format!("Fresh {id}."),
            id,
            speaker: None,
            count: false,
            choices: Vec::new(),
        }
    }

    /// Visits every statement, parents before their nested statements.
    pub fn visit(&self, visitor: &mut dyn FnMut(&Stmt)) {
        fn walk(statements: &[Stmt], visitor: &mut dyn FnMut(&Stmt)) {
            for statement in statements {
                visitor(statement);
                match statement {
                    Stmt::If {
                        then, otherwise, ..
                    } => {
                        walk(then, visitor);
                        walk(otherwise, visitor);
                    }
                    Stmt::Match { arms, .. } => {
                        for arm in arms {
                            walk(&arm.body, visitor);
                        }
                    }
                    _ => {}
                }
            }
        }
        for file in &self.files {
            for block in &file.blocks {
                walk(&block.body, visitor);
            }
        }
    }

    /// Visits every statement mutably, in the same order as [`Self::visit`].
    pub fn visit_mut(&mut self, visitor: &mut dyn FnMut(&mut Stmt)) {
        fn walk(statements: &mut [Stmt], visitor: &mut dyn FnMut(&mut Stmt)) {
            for statement in statements {
                visitor(statement);
                match statement {
                    Stmt::If {
                        then, otherwise, ..
                    } => {
                        walk(then, visitor);
                        walk(otherwise, visitor);
                    }
                    Stmt::Match { arms, .. } => {
                        for arm in arms {
                            walk(&mut arm.body, visitor);
                        }
                    }
                    _ => {}
                }
            }
        }
        for file in &mut self.files {
            for block in &mut file.blocks {
                walk(&mut block.body, visitor);
            }
        }
    }

    /// Visits every statement list: block bodies and the bodies nested in
    /// `:if`, `:else` and `:case`, parents first.
    pub fn visit_lists_mut(&mut self, visitor: &mut dyn FnMut(&mut Vec<Stmt>)) {
        fn walk(statements: &mut Vec<Stmt>, visitor: &mut dyn FnMut(&mut Vec<Stmt>)) {
            visitor(statements);
            for statement in statements {
                match statement {
                    Stmt::If {
                        then, otherwise, ..
                    } => {
                        walk(then, visitor);
                        walk(otherwise, visitor);
                    }
                    Stmt::Match { arms, .. } => {
                        for arm in arms {
                            walk(&mut arm.body, visitor);
                        }
                    }
                    _ => {}
                }
            }
        }
        for file in &mut self.files {
            for block in &mut file.blocks {
                walk(&mut block.body, visitor);
            }
        }
    }
}

fn divert_target(target: Target, current: usize, positions: &[(usize, usize)]) -> Target {
    let later = positions.get(current + 1..).unwrap_or_default();
    match target {
        Target::Block {
            file: RAW_FILE,
            block,
        } if !later.is_empty() => {
            let (file, block) = later[block % later.len()];
            Target::Block { file, block }
        }
        Target::Block { file, block } if later.contains(&(file, block)) => target,
        _ => Target::End,
    }
}
