use super::DiagnosticExplanation;
use crate::DiagnosticCategory;

pub(super) const EXPLANATIONS: &[DiagnosticExplanation] = &[
    DiagnosticExplanation::new(
        "RECITE_PROSE001",
        DiagnosticCategory::Prose,
        "A word in dialogue or choice prose is not in the prose dictionary.",
        &[
            "The word is misspelled.",
            "The word is a character name, place or invented term missing from the project dictionary.",
        ],
        &[
            "Apply a suggested spelling, or add the word to a `[prose].dictionaries` file.",
            "Add the line's speaker or stable ID to `[prose]` ignore settings for intentional nonstandard speech.",
        ],
    ),
    DiagnosticExplanation::new(
        "RECITE_PROSE002",
        DiagnosticCategory::Prose,
        "The prose checker found a likely grammar, usage or punctuation problem in dialogue or choice prose.",
        &[
            "A typo, agreement or article error in author-facing text.",
            "Intentional dialogue grammar that the checker cannot distinguish from a mistake.",
        ],
        &[
            "Apply the suggested correction or rewrite the sentence.",
            "Suppress the reported rule with `[prose].ignore_rules`, or the speaker or stable ID with `[prose].ignore_speakers` or `[prose].ignore_ids`.",
        ],
    ),
    DiagnosticExplanation::new(
        "RECITE_PROSE003",
        DiagnosticCategory::Prose,
        "The prose checker suggests a style or readability improvement in dialogue or choice prose.",
        &["Redundant, wordy or unusually formatted phrasing."],
        &["Apply the suggestion, or suppress the reported rule with `[prose].ignore_rules`."],
    ),
    DiagnosticExplanation::new(
        "RECITE_PROSE101",
        DiagnosticCategory::Prose,
        "The project's `[prose].language` is not a language the prose checker supports.",
        &["The language tag is misspelled or names a language without a bundled dictionary."],
        &["Use `en-US`, `en-GB`, `en-AU`, `en-CA` or `en-IN`."],
    ),
    DiagnosticExplanation::new(
        "RECITE_PROSE102",
        DiagnosticCategory::Prose,
        "A project prose dictionary could not be read.",
        &[
            "The file is not UTF-8 text, is a directory, or is not readable.",
            "The path escapes the project root.",
        ],
        &[
            "Point `[prose].dictionaries` at project-relative UTF-8 text files with one word per line.",
        ],
    ),
    DiagnosticExplanation::new(
        "RECITE_PROSE103",
        DiagnosticCategory::Prose,
        "This Recite build does not include a prose checker.",
        &["The binary was built without the `prose` Cargo feature."],
        &["Use an official Recite build, or rebuild with the default features enabled."],
    ),
];
