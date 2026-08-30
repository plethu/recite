/// A compiler-relevant class whose source recovery may be incomplete.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SourceRecoveryClass {
    AstStructure,
    BlockDefinitions,
    BlockReferences,
    StableIds,
    Metadata,
    ConditionFunctions,
    EffectFunctions,
    InlineMarkup,
}

/// Typed structural evidence produced by parsing and lowering recovery.
///
/// A class is incomplete only when the parser or lowerer could not prove that
/// it saw every source construct relevant to that class.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct SourceRecovery {
    ast_structure: bool,
    block_definitions: bool,
    block_references: bool,
    stable_ids: bool,
    metadata: bool,
    condition_functions: bool,
    effect_functions: bool,
    inline_markup: bool,
}

impl SourceRecovery {
    /// Recovery evidence for a source file with no known gaps.
    #[must_use]
    pub const fn complete() -> Self {
        Self {
            ast_structure: true,
            block_definitions: true,
            block_references: true,
            stable_ids: true,
            metadata: true,
            condition_functions: true,
            effect_functions: true,
            inline_markup: true,
        }
    }

    #[must_use]
    pub const fn ast_structure(self) -> bool {
        self.ast_structure
    }
    #[must_use]
    pub const fn block_definitions(self) -> bool {
        self.block_definitions
    }
    #[must_use]
    pub const fn block_references(self) -> bool {
        self.block_references
    }
    #[must_use]
    pub const fn stable_ids(self) -> bool {
        self.stable_ids
    }
    #[must_use]
    pub const fn metadata(self) -> bool {
        self.metadata
    }
    #[must_use]
    pub const fn condition_functions(self) -> bool {
        self.condition_functions
    }
    #[must_use]
    pub const fn effect_functions(self) -> bool {
        self.effect_functions
    }
    #[must_use]
    pub const fn inline_markup(self) -> bool {
        self.inline_markup
    }

    /// Marks one class as unable to support sound complete conclusions.
    pub const fn mark(&mut self, class: SourceRecoveryClass) {
        match class {
            SourceRecoveryClass::AstStructure => self.ast_structure = false,
            SourceRecoveryClass::BlockDefinitions => self.block_definitions = false,
            SourceRecoveryClass::BlockReferences => self.block_references = false,
            SourceRecoveryClass::StableIds => self.stable_ids = false,
            SourceRecoveryClass::Metadata => self.metadata = false,
            SourceRecoveryClass::ConditionFunctions => self.condition_functions = false,
            SourceRecoveryClass::EffectFunctions => self.effect_functions = false,
            SourceRecoveryClass::InlineMarkup => self.inline_markup = false,
        }
    }
}

impl Default for SourceRecovery {
    fn default() -> Self {
        Self::complete()
    }
}
