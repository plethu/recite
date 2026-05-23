use recite_core::{
    Block, BlockId, BlockReference, Choice, Diagnostic, DiagnosticCode, DiagnosticSeverity,
    EffectMode, Line, LineId, RelatedSpan, SchemaTypeRef, SourceFile, SourceSpan, Statement,
};

pub(crate) const MISSING_LINE_ID: &str = "RECITE_ID001";
pub(crate) const MISSING_CHOICE_ID: &str = "RECITE_ID002";
pub(crate) const DUPLICATE_LINE_ID: &str = "RECITE_ID003";
pub(crate) const DUPLICATE_CHOICE_ID: &str = "RECITE_ID004";
pub(crate) const MISSING_DEFAULT_BLOCK: &str = "RECITE_VALIDATE005";
pub(crate) const AMBIGUOUS_DEFAULT_BLOCK: &str = "RECITE_VALIDATE006";
pub(crate) const UNKNOWN_BLOCK_REFERENCE: &str = "RECITE_VALIDATE007";
pub(crate) const INVALID_SOURCE_SPAN: &str = "RECITE_VALIDATE008";
pub(crate) const DUPLICATE_BLOCK_ID: &str = "RECITE_VALIDATE009";
pub(crate) const DUPLICATE_SOURCE_PATH: &str = "RECITE_VALIDATE010";
pub(crate) const AMBIGUOUS_COMPILED_BLOCK_ID: &str = "RECITE_VALIDATE011";
pub(crate) const MISSING_CHOICE_TARGET: &str = "RECITE_VALIDATE012";
pub(crate) const UNSUPPORTED_LINE_CHILD_STATEMENT: &str = "RECITE_VALIDATE013";
pub(crate) const UNSUPPORTED_CHOICE_CHILD_STATEMENT: &str = "RECITE_VALIDATE014";
pub(crate) const UNKNOWN_CHOICE_ECHO_LINE: &str = "RECITE_VALIDATE015";
pub(crate) const NON_FINITE_FLOAT_VALUE: &str = "RECITE_VALIDATE016";
pub(crate) const UNKNOWN_EFFECT_FUNCTION: &str = "RECITE_VALIDATE017";
pub(crate) const WRONG_EFFECT_ARITY: &str = "RECITE_VALIDATE018";
pub(crate) const WRONG_EFFECT_ARGUMENT_TYPE: &str = "RECITE_VALIDATE019";
pub(crate) const UNSUPPORTED_EFFECT_MODE: &str = "RECITE_VALIDATE020";
pub(crate) const INVALID_EFFECT_ARGUMENT_VALUE: &str = "RECITE_VALIDATE021";

pub(crate) fn missing_line_id(line: &Line) -> Diagnostic {
    diagnostic(
        MISSING_LINE_ID,
        "line header must include a stable line id",
        line.span.clone(),
    )
    .with_help("add a stable author-visible ID to the line header")
}

pub(crate) fn missing_choice_id(choice: &Choice) -> Diagnostic {
    diagnostic(
        MISSING_CHOICE_ID,
        "choice header must include a stable choice id",
        choice.span.clone(),
    )
    .with_help("add a stable author-visible ID to the choice header")
}

pub(crate) fn duplicate_line_id(line: &Line, first_span: SourceSpan) -> Diagnostic {
    let id = line.id.as_ref().expect("duplicate line IDs have an ID");
    diagnostic(
        DUPLICATE_LINE_ID,
        format!("duplicate localisable id `{id}` on line"),
        line.span.clone(),
    )
    .with_related([RelatedSpan::new(first_span, "first localisable ID is here")])
    .with_help("rename one of the duplicate localisable IDs")
}

pub(crate) fn duplicate_choice_id(choice: &Choice, first_span: SourceSpan) -> Diagnostic {
    let id = choice.id.as_ref().expect("duplicate choice IDs have an ID");
    diagnostic(
        DUPLICATE_CHOICE_ID,
        format!("duplicate localisable id `{id}` on choice"),
        choice.span.clone(),
    )
    .with_related([RelatedSpan::new(first_span, "first localisable ID is here")])
    .with_help("rename one of the duplicate localisable IDs")
}

pub(crate) fn missing_default_block(span: SourceSpan) -> Diagnostic {
    diagnostic(
        MISSING_DEFAULT_BLOCK,
        "project must declare exactly one default block",
        span,
    )
    .with_help("mark one block header with `default`")
}

pub(crate) fn ambiguous_default_block(block: &Block, first: &Block) -> Diagnostic {
    diagnostic(
        AMBIGUOUS_DEFAULT_BLOCK,
        format!("block `{}` is another default block", block.id),
        block.span.clone(),
    )
    .with_related([RelatedSpan::new(
        first.span.clone(),
        "first default block is here",
    )])
    .with_help("keep exactly one block marked `default`")
}

pub(crate) fn unknown_block_reference(reference: &BlockReference, span: SourceSpan) -> Diagnostic {
    diagnostic(
        UNKNOWN_BLOCK_REFERENCE,
        format!("unknown block reference `{}`", display_reference(reference)),
        span,
    )
}

pub(crate) fn invalid_source_span(span: SourceSpan, owner: &str, detail: &str) -> Diagnostic {
    diagnostic(
        INVALID_SOURCE_SPAN,
        format!("invalid source span for {owner}: {detail}"),
        span,
    )
}

pub(crate) fn duplicate_block_id(
    block_id: &BlockId,
    span: SourceSpan,
    first_span: SourceSpan,
) -> Diagnostic {
    diagnostic(
        DUPLICATE_BLOCK_ID,
        format!("duplicate block id `{block_id}`"),
        span,
    )
    .with_related([RelatedSpan::new(first_span, "first block ID is here")])
    .with_help("rename one of the duplicate block IDs")
}

pub(crate) fn duplicate_source_path(
    source_file: &SourceFile,
    first_span: SourceSpan,
) -> Diagnostic {
    diagnostic(
        DUPLICATE_SOURCE_PATH,
        format!("duplicate source path `{}`", source_file.path),
        first_span_for(source_file),
    )
    .with_related([RelatedSpan::new(
        first_span,
        "first source file with this path is here",
    )])
    .with_help("compile each source path once")
}

pub(crate) fn ambiguous_compiled_block_id(
    block_id: &BlockId,
    span: SourceSpan,
    first_span: SourceSpan,
) -> Diagnostic {
    diagnostic(
        AMBIGUOUS_COMPILED_BLOCK_ID,
        format!("compiled block id `{block_id}` must be globally unique"),
        span,
    )
    .with_related([RelatedSpan::new(
        first_span,
        "first compiled block ID is here",
    )])
    .with_help("rename one block or split the runtime lookup contract in a future format version")
}

pub(crate) fn missing_choice_target(choice: &Choice) -> Diagnostic {
    diagnostic(
        MISSING_CHOICE_TARGET,
        "choice must target a block or END before it can be compiled",
        choice.span.clone(),
    )
    .with_help("add a choice body divert such as `-> next_block` or `-> END`")
}

pub(crate) fn unsupported_line_child_statement(line: &Line, statement: &Statement) -> Diagnostic {
    diagnostic(
        UNSUPPORTED_LINE_CHILD_STATEMENT,
        format!(
            "line `{}` contains a nested {} statement that v0 compiled prompts cannot represent",
            display_optional_line_id(line),
            display_statement_kind(statement),
        ),
        statement_span(statement).clone(),
    )
    .with_related([RelatedSpan::new(
        line.span.clone(),
        "line containing the unsupported nested statement is here",
    )])
    .with_help("keep only nested choices under prompt lines for v0 compiled assets")
}

pub(crate) fn unsupported_choice_child_statement(
    choice: &Choice,
    statement: &Statement,
) -> Diagnostic {
    diagnostic(
        UNSUPPORTED_CHOICE_CHILD_STATEMENT,
        format!(
            "choice `{}` contains a nested {} statement that v0 compiled choices cannot represent",
            display_optional_choice_id(choice),
            display_statement_kind(statement),
        ),
        statement_span(statement).clone(),
    )
    .with_related([RelatedSpan::new(
        choice.span.clone(),
        "choice containing the unsupported nested statement is here",
    )])
    .with_help("keep choice bodies to text and one target divert for v0 compiled assets")
}

pub(crate) fn unknown_choice_echo_line(choice: &Choice, line_id: &LineId) -> Diagnostic {
    diagnostic(
        UNKNOWN_CHOICE_ECHO_LINE,
        format!("choice echo references unknown line id `{line_id}`"),
        choice.span.clone(),
    )
    .with_help("use an existing line ID, `echo=selected_text`, or `echo=none`")
}

pub(crate) fn non_finite_float_value(
    span: SourceSpan,
    owner: impl std::fmt::Display,
) -> Diagnostic {
    diagnostic(
        NON_FINITE_FLOAT_VALUE,
        format!("{owner} contains a non-finite float value"),
        span,
    )
    .with_help("use a finite number so MessagePack and inspection JSON stay equivalent")
}

pub(crate) fn unknown_effect_function(function: &str, span: SourceSpan) -> Diagnostic {
    diagnostic(
        UNKNOWN_EFFECT_FUNCTION,
        format!("unknown effect function `{function}`"),
        span,
    )
    .with_help("declare the effect in the project schema manifest")
}

pub(crate) fn wrong_effect_arity(
    function: &str,
    expected: usize,
    actual: usize,
    span: SourceSpan,
) -> Diagnostic {
    diagnostic(
        WRONG_EFFECT_ARITY,
        format!(
            "effect `{function}` expects {expected} argument{}, but got {actual}",
            if expected == 1 { "" } else { "s" }
        ),
        span,
    )
    .with_help("match the effect parameters declared in the project schema manifest")
}

pub(crate) fn wrong_effect_argument_type(
    function: &str,
    index: usize,
    expected: &SchemaTypeRef,
    actual: &str,
    span: SourceSpan,
) -> Diagnostic {
    diagnostic(
        WRONG_EFFECT_ARGUMENT_TYPE,
        format!(
            "argument {} for effect `{function}` expects {}, but got {actual}",
            index + 1,
            display_schema_type_ref(expected),
        ),
        span,
    )
}

pub(crate) fn unsupported_effect_mode(
    function: &str,
    mode: EffectMode,
    span: SourceSpan,
) -> Diagnostic {
    diagnostic(
        UNSUPPORTED_EFFECT_MODE,
        format!(
            "effect `{function}` does not support {} mode",
            display_effect_mode(mode)
        ),
        span,
    )
    .with_help("use a mode declared for this effect in the project schema manifest")
}

pub(crate) fn invalid_effect_argument_value(
    function: &str,
    index: usize,
    expected: &SchemaTypeRef,
    value: &str,
    span: SourceSpan,
) -> Diagnostic {
    diagnostic(
        INVALID_EFFECT_ARGUMENT_VALUE,
        format!(
            "argument {} for effect `{function}` uses unknown {} value `{value}`",
            index + 1,
            display_schema_type_ref(expected),
        ),
        span,
    )
    .with_help("use a value exported in the project schema manifest")
}

fn diagnostic(code: &str, message: impl Into<String>, span: SourceSpan) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::new(code).expect("compiler diagnostic codes are static and namespaced"),
        DiagnosticSeverity::Error,
        message,
        span,
    )
}

fn display_reference(reference: &BlockReference) -> String {
    match &reference.file {
        Some(file) => format!("{file}::{}", reference.block_id),
        None => reference.block_id.to_string(),
    }
}

fn first_span_for(source_file: &SourceFile) -> SourceSpan {
    source_file.blocks.first().map_or_else(
        || {
            SourceSpan::point(
                source_file.path.clone(),
                recite_core::SourcePosition::new(1, 1).expect("1:1 is a valid source position"),
            )
        },
        |block| block.span.clone(),
    )
}

fn display_optional_line_id(line: &Line) -> String {
    line.id
        .as_ref()
        .map_or_else(|| "<missing>".to_owned(), ToString::to_string)
}

fn display_optional_choice_id(choice: &Choice) -> String {
    choice
        .id
        .as_ref()
        .map_or_else(|| "<missing>".to_owned(), ToString::to_string)
}

fn display_statement_kind(statement: &Statement) -> &'static str {
    match statement {
        Statement::Line(_) => "line",
        Statement::Choice(_) => "choice",
        Statement::Divert(_) => "divert",
        Statement::If(_) => "if",
        Statement::Match(_) => "match",
        Statement::Effect(_) => "effect",
        Statement::Comment(_) => "comment",
    }
}

fn display_effect_mode(mode: EffectMode) -> &'static str {
    match mode {
        EffectMode::Deferred => "deferred",
        EffectMode::Immediate => "immediate",
        EffectMode::Blocking => "blocking",
    }
}

fn display_schema_type_ref(type_ref: &SchemaTypeRef) -> String {
    match type_ref {
        SchemaTypeRef::String => "string".to_owned(),
        SchemaTypeRef::Int => "int".to_owned(),
        SchemaTypeRef::Float => "float".to_owned(),
        SchemaTypeRef::Bool => "bool".to_owned(),
        SchemaTypeRef::Speaker => "speaker".to_owned(),
        SchemaTypeRef::Enum(name) => format!("enum:{name}"),
        SchemaTypeRef::Registry(name) => format!("registry:{name}"),
    }
}

fn statement_span(statement: &Statement) -> &SourceSpan {
    match statement {
        Statement::Line(line) => &line.span,
        Statement::Choice(choice) => &choice.span,
        Statement::Divert(divert) => &divert.span,
        Statement::If(branch) => &branch.span,
        Statement::Match(branch) => &branch.span,
        Statement::Effect(effect) => &effect.span,
        Statement::Comment(comment) => &comment.span,
    }
}
