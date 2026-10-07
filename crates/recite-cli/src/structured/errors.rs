use serde::Serialize;

#[derive(Serialize)]
#[cfg_attr(test, derive(strum::EnumIter))]
#[serde(rename_all = "snake_case")]
pub(crate) enum ErrorCategory {
    Input,
    Io,
    Schema,
    Compilation,
    Asset,
    Fixture,
    Runtime,
    Localisation,
    Configuration,
    Serialization,
    Project,
    Watch,
    Benchmark,
    Unsupported,
    Internal,
}

#[derive(Serialize)]
#[cfg_attr(test, derive(strum::EnumIter))]
#[serde(rename_all = "snake_case")]
pub(crate) enum ErrorCode {
    Import,
    ImportJson,
    CoreValue,
    Compile,
    CompiledValue,
    DecodeAsset,
    Diagnostics,
    DiagnosticRendering,
    DialogueCatalogConflict,
    DialogueCatalogPluralFormsConflict,
    DialogueCatalogMalformed,
    DialogueCatalogMissingLocale,
    DialogueCatalogSpecInvalid,
    DialogueLocaleInvalid,
    DiagnosticCodeMalformed,
    DiagnosticCodeUnknown,
    FixtureChoiceIndexOutOfRange,
    FixtureChoiceNotInPrompt,
    AmbiguousFixtureChoice,
    FixtureToml,
    AssetMetadata,
    AssetNotFile,
    Io,
    MalformedCompiledAsset,
    MissingPath,
    InvalidProjectRoot,
    MissingFixtureChoice,
    NoInputs,
    OutputOverwritesInput,
    PlayEof,
    PlayInvalidInput,
    PlayInterrupted,
    PlayTuiRequiresTerminal,
    Read,
    ReadDirectory,
    Runtime,
    Preview,
    BlockingEffectNeedsAcknowledgement,
    Bench,
    Benchmark,
    BenchJson,
    TraceJson,
    SchemaInspection,
    UserConfig,
    ProjectDiscovery,
    ProjectSchema,
    UiCatalog,
    Watch,
    WatchCoordinator,
    WatchRecovery,
    Write,
    WatchPreparation,
    WatchPublisher,
}

#[derive(Clone, Copy, Serialize)]
#[cfg_attr(test, derive(strum::EnumIter))]
#[serde(rename_all = "snake_case")]
pub(crate) enum ErrorOperation {
    Validate,
    Compile,
    Extract,
    Run,
    Trace,
    Watch,
    LoadAsset,
    LoadCatalog,
    LoadFixture,
    InspectAsset,
    ResolvePath,
    CollectInputs,
    WriteOutput,
    Read,
    ReadDirectory,
    AcknowledgeEffect,
    SelectFixtureChoice,
    Write,
    DiscoverProject,
    StartWatcher,
    WatchProject,
    Build,
    ReadProjectInput,
    ResolveSchema,
    LoadSchema,
    PrepareInputs,
    ValidateProject,
    PrepareRequest,
    PrepareTargets,
    PrepareProject,
    PreparePublisher,
    ResolveProjectRoot,
    ValidateTarget,
    ExportSchema,
    Dispatch,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum ErrorDetails {
    FixtureChoice {
        choice: String,
        prompt_keys: Vec<String>,
    },
    FixtureChoiceIndex {
        index: usize,
        choice_count: usize,
        prompt_keys: Vec<String>,
    },
    AmbiguousFixture {
        block: String,
        prompt_count: usize,
    },
    MissingFixtureChoice {
        prompt_keys: Vec<String>,
    },
    BlockingEffect {
        effect: String,
    },
    Locale {
        field: &'static str,
        locale: String,
    },
    CatalogSpec {
        spec: String,
    },
    Watch {
        kind: &'static str,
    },
    WatchTarget {
        kind: &'static str,
        target: String,
    },
}

#[derive(Serialize)]
pub(crate) struct StructuredError {
    pub(crate) category: ErrorCategory,
    pub(crate) code: ErrorCode,
    pub(crate) operation: ErrorOperation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) path: Option<crate::schema_inspection::MachinePathProjection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) related_path: Option<crate::schema_inspection::MachinePathProjection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) details: Option<ErrorDetails>,
}
