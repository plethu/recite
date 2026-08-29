use super::{DiagnosticArgumentSpec, DiagnosticArgumentType, DiagnosticPresentationContract};

const NO_ARGUMENTS: &[DiagnosticArgumentSpec] = &[];
const FIELD: &[DiagnosticArgumentSpec] = &[DiagnosticArgumentSpec::new(
    "field",
    DiagnosticArgumentType::String,
)];
const ESCAPE: &[DiagnosticArgumentSpec] = &[DiagnosticArgumentSpec::new(
    "escape",
    DiagnosticArgumentType::String,
)];
const VALUE: &[DiagnosticArgumentSpec] = &[DiagnosticArgumentSpec::new(
    "value",
    DiagnosticArgumentType::String,
)];
const CONTEXT: &[DiagnosticArgumentSpec] = &[DiagnosticArgumentSpec::new(
    "context",
    DiagnosticArgumentType::String,
)];
const SOURCE_KEY: &[DiagnosticArgumentSpec] = &[
    DiagnosticArgumentSpec::new("context", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("source_text", DiagnosticArgumentType::String),
];
const DETAIL: &[DiagnosticArgumentSpec] = &[DiagnosticArgumentSpec::new(
    "detail",
    DiagnosticArgumentType::String,
)];
const TAG: &[DiagnosticArgumentSpec] = &[DiagnosticArgumentSpec::new(
    "tag",
    DiagnosticArgumentType::String,
)];
const MARKUP_ATTRIBUTE_CHANGE: &[DiagnosticArgumentSpec] = &[
    DiagnosticArgumentSpec::new("tag", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("expected", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("actual", DiagnosticArgumentType::String),
];
const EXPECTED: &[DiagnosticArgumentSpec] = &[DiagnosticArgumentSpec::new(
    "expected",
    DiagnosticArgumentType::Integer,
)];
const EXPECTED_ACTUAL: &[DiagnosticArgumentSpec] = &[
    DiagnosticArgumentSpec::new("expected", DiagnosticArgumentType::Integer),
    DiagnosticArgumentSpec::new("actual", DiagnosticArgumentType::Integer),
];
const KEYWORD: &[DiagnosticArgumentSpec] = &[DiagnosticArgumentSpec::new(
    "keyword",
    DiagnosticArgumentType::String,
)];
const LINE: &[DiagnosticArgumentSpec] = &[DiagnosticArgumentSpec::new(
    "line",
    DiagnosticArgumentType::String,
)];
const KEY: &[DiagnosticArgumentSpec] = &[DiagnosticArgumentSpec::new(
    "key",
    DiagnosticArgumentType::String,
)];

// PARSE034 has one presentation for each structural cause. In particular,
// the no-argument causes do not share a generic default resource: keeping
// these IDs distinct preserves the useful compatibility message from the PO
// parser while still allowing callers to supply typed dynamic values for the
// other causes.
const EXPECTED_DIRECTIVE: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_PARSE034",
    "diagnostic-parse-034-expected-directive",
    NO_ARGUMENTS,
);
const EXPECTED_QUOTED_STRING: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_PARSE034",
    "diagnostic-parse-034-expected-quoted-string",
    NO_ARGUMENTS,
);
const MISSING_FIELD: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_PARSE034",
    "diagnostic-parse-034-missing-field",
    FIELD,
);
const DUPLICATE_FIELD: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_PARSE034",
    "diagnostic-parse-034-duplicate-field",
    FIELD,
);
const QUOTED_WITHOUT_FIELD: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_PARSE034",
    "diagnostic-parse-034-quoted-without-field",
    NO_ARGUMENTS,
);
const UNEXPECTED_TRAILING_TEXT: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new(
        "RECITE_PARSE034",
        "diagnostic-parse-034-unexpected-trailing-text",
        NO_ARGUMENTS,
    );
const UNTERMINATED_QUOTED_STRING: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new(
        "RECITE_PARSE034",
        "diagnostic-parse-034-unterminated-quoted-string",
        NO_ARGUMENTS,
    );
const UNSUPPORTED_ESCAPE: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_PARSE034",
    "diagnostic-parse-034-unsupported-escape",
    ESCAPE,
);
const INVALID_FIELD_ORDER: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_PARSE034",
    "diagnostic-parse-034-invalid-field-order",
    VALUE,
);

// ID034 and ID035 retain their default code presentation IDs because their
// signatures are stable and sufficient for every producer call site.
const INVALID_STABLE_ID: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_ID034", "diagnostic-id-034", CONTEXT);
const DUPLICATE_KEY: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_ID035", "diagnostic-id-035", SOURCE_KEY);

const PLACEHOLDER_MISMATCH: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_VALIDATE042", "diagnostic-validate-042", DETAIL);

// InvalidPluralArms has finite causes with different typed arguments. Do not
// expose the parser's free-form detail as the primary contract: the values
// are either a known structural sentence, an arm keyword, or numeric counts.
const PLURAL_CONTIGUOUS_ARMS: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_VALIDATE043",
    "diagnostic-validate-043-contiguous-arms",
    NO_ARGUMENTS,
);
const PLURAL_EXPECTED_ARM: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_VALIDATE043",
    "diagnostic-validate-043-expected-arm",
    EXPECTED,
);
const PLURAL_REQUIRES_SOURCE: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_VALIDATE043",
    "diagnostic-validate-043-requires-plural-source",
    NO_ARGUMENTS,
);
const PLURAL_COUNT: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_VALIDATE043",
    "diagnostic-validate-043-count",
    EXPECTED_ACTUAL,
);
const PLURAL_INVALID_ARM: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_VALIDATE043",
    "diagnostic-validate-043-invalid-arm",
    KEYWORD,
);

// InvalidHeader also has finite structural causes. A header line/key remains
// a direct string argument; the other cases are deliberately no-argument
// resources rather than a generic `detail` presentation.
const MULTIPLE_HEADERS: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_VALIDATE044",
    "diagnostic-validate-044-multiple-headers",
    NO_ARGUMENTS,
);
const HEADER_MISSING_COLON: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_VALIDATE044",
    "diagnostic-validate-044-missing-colon",
    LINE,
);
const HEADER_DUPLICATE_OR_EMPTY: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new(
        "RECITE_VALIDATE044",
        "diagnostic-validate-044-duplicate-or-empty",
        KEY,
    );
const INVALID_PLURAL_FORMS: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_VALIDATE044",
    "diagnostic-validate-044-invalid-plural-forms",
    NO_ARGUMENTS,
);
const INVALID_PLURAL_RULE: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_VALIDATE044",
    "diagnostic-validate-044-invalid-plural-rule",
    DETAIL,
);
const PLURAL_HEADER_REQUIRED: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_VALIDATE044",
    "diagnostic-validate-044-plural-header-required",
    NO_ARGUMENTS,
);
const MARKUP_ATTRIBUTE: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_VALIDATE047",
    "diagnostic-validate-047",
    MARKUP_ATTRIBUTE_CHANGE,
);
const MARKUP_NEW_TAG: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_VALIDATE048", "diagnostic-validate-048", TAG);
const MARKUP_MISSING_TAG: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_VALIDATE049", "diagnostic-validate-049", TAG);

static CONTRACTS: &[&DiagnosticPresentationContract] = &[
    &EXPECTED_DIRECTIVE,
    &EXPECTED_QUOTED_STRING,
    &MISSING_FIELD,
    &DUPLICATE_FIELD,
    &QUOTED_WITHOUT_FIELD,
    &UNEXPECTED_TRAILING_TEXT,
    &UNTERMINATED_QUOTED_STRING,
    &UNSUPPORTED_ESCAPE,
    &INVALID_FIELD_ORDER,
    &INVALID_STABLE_ID,
    &DUPLICATE_KEY,
    &PLACEHOLDER_MISMATCH,
    &PLURAL_CONTIGUOUS_ARMS,
    &PLURAL_EXPECTED_ARM,
    &PLURAL_REQUIRES_SOURCE,
    &PLURAL_COUNT,
    &PLURAL_INVALID_ARM,
    &MULTIPLE_HEADERS,
    &HEADER_MISSING_COLON,
    &HEADER_DUPLICATE_OR_EMPTY,
    &INVALID_PLURAL_FORMS,
    &INVALID_PLURAL_RULE,
    &PLURAL_HEADER_REQUIRED,
    &MARKUP_ATTRIBUTE,
    &MARKUP_NEW_TAG,
    &MARKUP_MISSING_TAG,
];

pub(super) fn contracts() -> impl Iterator<Item = &'static DiagnosticPresentationContract> {
    CONTRACTS.iter().copied()
}
