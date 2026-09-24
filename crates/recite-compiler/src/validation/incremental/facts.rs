//! Compact project inputs. Prose and local schema syntax do not survive analysis.
use crate::{validation::ValidationInput, validation::ValidationParticipation};
use recite_core::{
    BlockId, ChoiceId, LineId, SourceId, SourceSpan,
    ast::{
        Block, Choice, ChoiceEcho, Divert, DivertTarget, Line, SourceFile, SourceText, Statement,
    },
};

#[derive(Debug, PartialEq)]
pub(crate) struct ProjectFacts {
    pub(super) path: String,
    pub(super) participation: ValidationParticipation,
    pub(super) blocks: Box<[BlockFact]>,
    pub(super) passages: Box<[PassageFact]>,
    pub(super) references: Box<[Divert]>,
}
#[derive(Debug, PartialEq)]
pub(super) struct BlockFact {
    pub(super) id: BlockId,
    pub(super) default: bool,
    pub(super) span: SourceSpan,
}
#[derive(Debug, PartialEq)]
pub(super) struct PassageFact {
    pub(super) identity: Identity,
    pub(super) frozen: bool,
    pub(super) span: SourceSpan,
}
#[derive(Debug, PartialEq)]
pub(super) enum Identity {
    Line(Option<LineId>),
    Choice(Option<ChoiceId>, ChoiceEcho),
}
impl ProjectFacts {
    pub(crate) fn collect(input: ValidationInput<'_>) -> Self {
        let file = input.source_file();
        let mut passages = Vec::new();
        let mut references = Vec::new();
        file.visit_statements_depth_first(&mut |statement| match statement {
            Statement::Line(line) => passages.push(PassageFact {
                identity: Identity::Line(line.id.clone()),
                frozen: matches!(line.source_id, SourceId::Frozen { .. }),
                span: line.span.clone(),
            }),
            Statement::Choice(choice) => {
                passages.push(PassageFact {
                    identity: Identity::Choice(choice.id.clone(), choice.echo.clone()),
                    frozen: matches!(choice.source_id, SourceId::Frozen { .. }),
                    span: choice.span.clone(),
                });
                if let Some(target) = &choice.target {
                    references.push(Divert::new(target.target.clone(), target.span.clone()));
                }
            }
            Statement::Divert(divert) => references.push(divert.clone()),
            _ => {}
        });
        Self {
            path: file.path.clone(),
            participation: input.participation(),
            blocks: file
                .blocks
                .iter()
                .map(|block| BlockFact {
                    id: block.id.clone(),
                    default: block.is_default,
                    span: block.span.clone(),
                })
                .collect(),
            passages: passages.into_boxed_slice(),
            references: references.into_boxed_slice(),
        }
    }

    // Only the existing project validator consumes this temporary projection.
    // Flattening preserves passage order; local semantics use the original AST.
    pub(super) fn project_source(&self) -> SourceFile {
        let mut blocks: Vec<_> = self
            .blocks
            .iter()
            .map(|block| {
                Block::new(block.id.clone(), Vec::new(), block.span.clone())
                    .with_default(block.default)
            })
            .collect();
        if let Some(first) = blocks.first_mut() {
            first.statements = self
                .passages
                .iter()
                .map(|passage| {
                    let text = SourceText::new(String::new(), passage.span.clone());
                    match &passage.identity {
                        Identity::Line(id) => {
                            let mut line = Line::new(id.clone(), text, passage.span.clone());
                            if !passage.frozen {
                                line.source_id = SourceId::Missing;
                            }
                            Statement::Line(line)
                        }
                        Identity::Choice(id, echo) => {
                            let mut choice = Choice::new(id.clone(), text, passage.span.clone());
                            if !passage.frozen {
                                choice.source_id = SourceId::Missing;
                            }
                            choice.echo = echo.clone();
                            Statement::Choice(choice)
                        }
                    }
                })
                .chain(self.references.iter().cloned().map(Statement::Divert))
                .collect();
        }
        SourceFile::new(&self.path, blocks)
    }

    pub(super) fn same_block_targets(&self, other: &Self) -> bool {
        self.participation.block_definitions() == other.participation.block_definitions()
            && self
                .blocks
                .iter()
                .map(|block| &block.id)
                .eq(other.blocks.iter().map(|block| &block.id))
    }

    pub(super) fn exports(&self) -> impl Iterator<Item = Symbol> + '_ {
        let blocks = self
            .blocks
            .iter()
            .map(|block| Symbol::Block(block.id.clone()));
        let ids = self
            .passages
            .iter()
            .filter_map(|passage| match &passage.identity {
                Identity::Line(id) => id.as_ref().map(|id| Symbol::Id(id.as_str().to_owned())),
                Identity::Choice(id, _) => id.as_ref().map(|id| Symbol::Id(id.as_str().to_owned())),
            });
        blocks
            .chain(ids)
            .chain(std::iter::once(Symbol::Document(self.path.clone())))
            .chain(
                self.blocks
                    .iter()
                    .any(|b| b.default)
                    .then_some(Symbol::Default),
            )
    }

    pub(super) fn dependencies(&self) -> impl Iterator<Item = Symbol> + '_ {
        let echoes = self
            .passages
            .iter()
            .filter_map(|passage| match &passage.identity {
                Identity::Choice(_, ChoiceEcho::Line(id)) => {
                    Some(Symbol::Id(id.as_str().to_owned()))
                }
                _ => None,
            });
        let references = self
            .references
            .iter()
            .filter_map(|reference| match &reference.target {
                DivertTarget::Block(target) => Some(Symbol::Document(
                    target.file.clone().unwrap_or_else(|| self.path.clone()),
                )),
                DivertTarget::End => None,
            });
        echoes.chain(references)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(super) enum Symbol {
    Id(String),
    Block(BlockId),
    Document(String),
    Default,
}
