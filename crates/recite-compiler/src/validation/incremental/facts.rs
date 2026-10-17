mod positions;
mod regions;
#[cfg(test)]
mod tests;
pub(crate) use regions::FactRanges;
// Compact project inputs. Prose and local schema syntax do not survive analysis.
use crate::{validation::ValidationInput, validation::ValidationParticipation};
use recite_core::{
    BlockId, ChoiceId, LineId, SourceId, SourceSpan,
    ast::{Block, ChoiceEcho, Divert, DivertTarget, SourceFile, Statement},
};
use std::sync::Arc;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProjectFacts {
    pub(super) path: String,
    pub(super) participation: ValidationParticipation,
    pub(super) blocks: Arc<Vec<BlockFact>>,
    pub(super) passages: Arc<Vec<PassageFact>>,
    pub(super) references: Arc<Vec<Divert>>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct BlockFact {
    pub(super) id: BlockId,
    pub(super) default: bool,
    pub(super) span: SourceSpan,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct PassageFact {
    pub(super) identity: Identity,
    pub(super) frozen: bool,
    pub(super) span: SourceSpan,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum Identity {
    Line(Option<LineId>),
    Choice(Option<ChoiceId>, ChoiceEcho),
}
impl ProjectFacts {
    pub(super) fn shares_storage(&self, previous: &Self) -> bool {
        let Self {
            path,
            participation,
            blocks,
            passages,
            references,
        } = self;
        path == &previous.path
            && participation == &previous.participation
            && Arc::ptr_eq(blocks, &previous.blocks)
            && Arc::ptr_eq(passages, &previous.passages)
            && Arc::ptr_eq(references, &previous.references)
    }

    pub(super) fn same_non_reference_facts(&self, next: &Self) -> bool {
        // Exhaustive destructuring makes new fact families revisit this proof.
        let Self {
            path,
            participation,
            blocks,
            passages,
            references: _,
        } = self;
        path == &next.path
            && participation == &next.participation
            && blocks == &next.blocks
            && passages == &next.passages
    }

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
                .collect::<Vec<_>>()
                .into(),
            passages: Arc::new(passages),
            references: Arc::new(references),
        }
    }

    // Block headers retain the existing validator's participation and lookup
    // ownership; passages and references are borrowed from these compact facts.
    pub(super) fn project_header(&self) -> SourceFile {
        SourceFile::new(
            &self.path,
            self.blocks
                .iter()
                .map(|block| {
                    Block::new(block.id.clone(), Vec::new(), block.span.clone())
                        .with_default(block.default)
                })
                .collect(),
        )
    }

    pub(super) fn same_block_targets(&self, other: &Self) -> bool {
        self.participation.block_definitions() == other.participation.block_definitions()
            && self
                .blocks
                .iter()
                .map(|block| &block.id)
                .eq(other.blocks.iter().map(|block| &block.id))
    }

    // A conservative, allocation-free proof of unchanged export membership.
    // Other facts still update: echoes, participation and spans affect validation.
    pub(super) fn same_ordered_exports(&self, next: &Self) -> bool {
        self.path == next.path
            && self
                .blocks
                .iter()
                .map(|block| &block.id)
                .eq(next.blocks.iter().map(|block| &block.id))
            && (Arc::ptr_eq(&self.passages, &next.passages)
                || self.exported_ids().eq(next.exported_ids()))
            && self.blocks.iter().any(|block| block.default)
                == next.blocks.iter().any(|block| block.default)
    }

    fn exported_ids(&self) -> impl Iterator<Item = &str> {
        self.passages
            .iter()
            .filter_map(|passage| match &passage.identity {
                Identity::Line(id) => id.as_ref().map(|id| id.as_str()),
                Identity::Choice(id, _) => id.as_ref().map(|id| id.as_str()),
            })
    }

    pub(super) fn exports(&self) -> impl Iterator<Item = Symbol> + '_ {
        let blocks = self
            .blocks
            .iter()
            .map(|block| Symbol::Block(block.id.clone()));
        let ids = self.exported_ids().map(|id| Symbol::Id(id.to_owned()));
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
        echoes.chain(self.reference_dependencies())
    }

    pub(super) fn reference_dependencies(&self) -> impl Iterator<Item = Symbol> + '_ {
        self.references
            .iter()
            .filter_map(|reference| match &reference.target {
                DivertTarget::Block(target) => Some(Symbol::Document(
                    target.file.clone().unwrap_or_else(|| self.path.clone()),
                )),
                DivertTarget::End => None,
            })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(super) enum Symbol {
    Id(String),
    Block(BlockId),
    Document(String),
    Default,
}

impl ProjectFacts {
    /// Only choice-echo lookup depends on stable-ID completeness in other files.
    /// Local ID checks and membership invalidation have their own dependencies.
    pub(crate) fn depends_on_stable_completeness(&self) -> bool {
        self.passages.iter().any(|passage| {
            matches!(
                passage.identity,
                Identity::Choice(_, recite_core::ast::ChoiceEcho::Line(_))
            )
        })
    }
}
