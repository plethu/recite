use super::DiagnosticExplanation;
use crate::DiagnosticCategory;

pub(super) const EXPLANATIONS: &[DiagnosticExplanation] = &[
    DiagnosticExplanation::new(
        "RECITE_PARSE001",
        DiagnosticCategory::Parse,
        "The parser found source text that does not match Recite syntax.",
        &["A statement marker, indentation level, or directive is malformed."],
        &["Check the reported span and rewrite the line using the Recite source format."],
    ),
    DiagnosticExplanation::new(
        "RECITE_PARSE002",
        DiagnosticCategory::Parse,
        "A statement appears before any block header.",
        &["The file starts with prose or another statement before the first `:: block` header."],
        &[
            "Add the missing block header before the statement or move the statement into an existing block.",
        ],
    ),
    DiagnosticExplanation::new(
        "RECITE_PARSE003",
        DiagnosticCategory::Parse,
        "A block header is missing its block ID.",
        &["A `::` header was written without a following identifier."],
        &["Add the block ID after `::`."],
    ),
    DiagnosticExplanation::new(
        "RECITE_PARSE005",
        DiagnosticCategory::Parse,
        "A block header contains an empty block ID.",
        &["The header contains only whitespace where the block ID should be."],
        &["Replace the empty block ID with a valid block name."],
    ),
    DiagnosticExplanation::new(
        "RECITE_PARSE007",
        DiagnosticCategory::Parse,
        "A statement body mixes indentation styles or indentation widths.",
        &["Indented child lines under the same statement do not align consistently."],
        &["Make the nested body use one consistent indentation level."],
    ),
    DiagnosticExplanation::new(
        "RECITE_PARSE008",
        DiagnosticCategory::Parse,
        "A statement header field is malformed.",
        &[
            "A block, choice, metadata, or condition header contains a field the parser cannot read.",
        ],
        &["Rewrite the reported header field using the supported Recite statement syntax."],
    ),
    DiagnosticExplanation::new(
        "RECITE_PARSE010",
        DiagnosticCategory::Parse,
        "A divert header is missing its target.",
        &["A `->` divert was written without a following block ID, external target, or END."],
        &["Add the intended divert target after `->`."],
    ),
    DiagnosticExplanation::new(
        "RECITE_PARSE011",
        DiagnosticCategory::Parse,
        "A divert target is malformed.",
        &["The target after `->` does not match a valid block, external block, or END target."],
        &["Rewrite the divert target using Recite's supported target syntax."],
    ),
    DiagnosticExplanation::new(
        "RECITE_PARSE012",
        DiagnosticCategory::Parse,
        "An effect statement is malformed.",
        &["An effect function name, mode, or argument list is incomplete."],
        &["Rewrite the effect using the supported effect call syntax."],
    ),
    DiagnosticExplanation::new(
        "RECITE_PARSE013",
        DiagnosticCategory::Parse,
        "A condition expression is malformed.",
        &["A condition call, operator, grouping, or argument list is incomplete."],
        &["Rewrite the condition expression with valid Recite condition syntax."],
    ),
    DiagnosticExplanation::new(
        "RECITE_PARSE014",
        DiagnosticCategory::Parse,
        "A match case is malformed.",
        &["A case pattern or case body is incomplete."],
        &["Rewrite the case with a supported pattern and body."],
    ),
    DiagnosticExplanation::new(
        "RECITE_PARSE015",
        DiagnosticCategory::Parse,
        "An `else` clause appears where no matching `if` can own it.",
        &["The `else` is incorrectly indented or placed outside its conditional group."],
        &["Move the `else` under the intended `if` or remove it."],
    ),
    DiagnosticExplanation::new(
        "RECITE_PARSE016",
        DiagnosticCategory::Parse,
        "A `case` clause appears where no matching `match` can own it.",
        &["The `case` is incorrectly indented or placed outside its match group."],
        &["Move the `case` under the intended `match` or remove it."],
    ),
    DiagnosticExplanation::new(
        "RECITE_PARSE017",
        DiagnosticCategory::Parse,
        "Prose appears after a nested statement in the same body.",
        &[
            "A line body mixes nested statements and later prose where the parser cannot preserve ownership.",
        ],
        &["Move the prose before the nested statements or split it into a separate line body."],
    ),
    DiagnosticExplanation::new(
        "RECITE_PARSE018",
        DiagnosticCategory::Parse,
        "A choice has a trailing `if` clause that is not valid Recite syntax.",
        &["Condition syntax from another dialogue format was used after a choice."],
        &["Use Recite's supported `requires` availability clause instead."],
    ),
    DiagnosticExplanation::new(
        "RECITE_PARSE034",
        DiagnosticCategory::Parse,
        "A gettext PO record is not structurally valid.",
        &["A PO directive, quoted string, continuation, or field boundary is malformed."],
        &["Fix the reported PO record while preserving its surrounding comments and fields."],
    ),
];
