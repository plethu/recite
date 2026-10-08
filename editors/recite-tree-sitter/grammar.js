// Tree-sitter is an editor-facing syntax layer for Recite.
//
// This grammar deliberately does not model indentation ownership or any
// semantic rule. Rowan, the compiler, and the LSP remain authoritative for
// source recovery, stable IDs, references, schema, conditions, effects,
// markup, and match exhaustiveness.

const {
  directiveNonWhitespace,
  commentLine,
  blockStatement,
  lineStatement,
  choiceStatement,
  effectStatement,
  divertStatement,
  ifLine,
  matchLine,
  elseLine,
  caseLine,
  pluralLine,
  proseLine,
  sourceLines,
  finalLine,
} = require("./grammar-lines.js");

module.exports = grammar({
  name: "recite",

  // Whitespace and line endings are part of the grammar. Keeping them out of
  // `extras` makes the line-oriented shape useful to editor structural tools
  // without pretending that this grammar owns Recite's indentation semantics.
  extras: (_$) => [],

  // Incomplete or prose-like brackets remain editable text; completed tags
  // retain their name captures. The compiler owns markup validity.
  conflicts: ($) => [[$.markup_tag, $.prose_start], [$.markup_tag, $.prose_content]],

  rules: {
    source_file: ($) => seq(repeat(sourceLines($)), finalLine($)),

    blank_line: ($) => seq(optional($.indent), $.newline),
    comment_line: ($) => commentLine($, $.newline),
    block_statement: ($) => blockStatement($, $.newline),
    line_statement: ($) => lineStatement($, $.newline),
    choice_statement: ($) => choiceStatement($, $.newline),
    effect_statement: ($) => effectStatement($, $.newline),
    divert_statement: ($) => divertStatement($, $.newline),
    if_statement: ($) => ifLine($, $.newline),
    else_statement: ($) => elseLine($, $.newline),
    match_statement: ($) => matchLine($, $.newline),
    case_statement: ($) => caseLine($, $.newline),

    // `|` is syntax-only here. Whether it is the second source form of a
    // plural line is a compiler/parser decision, not a Tree-sitter decision.
    plural_line: ($) => pluralLine($, $.newline),
    prose_line: ($) => proseLine($, $.newline),

    _final_blank_line: ($) => $.indent,
    _final_comment_line: ($) => commentLine($),
    _final_block_statement: ($) => blockStatement($),
    _final_line_statement: ($) => lineStatement($),
    _final_choice_statement: ($) => choiceStatement($),
    _final_effect_statement: ($) => effectStatement($),
    _final_divert_statement: ($) => divertStatement($),
    _final_if_statement: ($) => ifLine($),
    _final_else_statement: ($) => elseLine($),
    _final_match_statement: ($) => matchLine($),
    _final_case_statement: ($) => caseLine($),
    _final_plural_line: ($) => pluralLine($),
    _final_prose_line: ($) => proseLine($),

    block_attribute: ($) =>
      seq(
        $.hspace,
        choice($.block_default, $.metadata_field),
      ),

    header_attribute: ($) =>
      seq(
        $.hspace,
        $.metadata_field,
      ),

    choice_attribute: ($) =>
      seq(
        $.hspace,
        choice($.requires_clause, $.reason_clause, $.metadata_field),
      ),

    requires_clause: ($) =>
      seq(
        field("key", $.requires_key),
        optional($.hspace),
        "=",
        optional($.hspace),
        field("value", $.grouped_value),
      ),

    reason_clause: ($) =>
      seq(
        field("key", $.reason_key),
        optional($.hspace),
        "=",
        optional($.hspace),
        field("value", $.value),
      ),

    metadata_field: ($) =>
      seq(
        field("key", $.metadata_key),
        optional($.hspace),
        "=",
        optional($.hspace),
        field("value", $.value),
      ),

    // A grouped value is intentionally permissive. It covers condition
    // operators (`and`, `or`, `not`) and typed bindings without deciding their
    // meaning. The semantic parser/compiler owns those rules.
    grouped_value: ($) =>
      seq(
        "(",
        optional($.hspace),
        $.expression_part,
        repeat(seq($.hspace, $.expression_part)),
        optional($.hspace),
        ")",
      ),

    expression_part: ($) =>
      choice(
        $.call,
        $.binding,
        $.array_value,
        $.string,
        $.number,
        $.boolean,
        $.runtime_binding,
        $.operator,
        $.symbol,
        $.grouped_value,
      ),

    call: ($) =>
      prec.right(seq(
        field("function", $.function_name),
        "(",
        optional($.arguments),
        optional($.hspace),
        // A call may be left open while an author is typing. Because the call
        // can end only at the current physical line, recovery cannot consume
        // the next statement's marker.
        optional(")"),
      )),

    arguments: ($) =>
      prec.right(seq(
        $.argument,
        repeat(seq(",", optional($.hspace), $.argument)),
      )),

    argument: ($) => $.value,

    binding: ($) =>
      seq(
        field("name", $.identifier),
        ":",
        field("type", $.type_name),
        "=",
        field("value", $.value),
      ),

    value: ($) =>
      choice(
        $.call,
        $.binding,
        $.array_value,
        $.string,
        $.number,
        $.boolean,
        $.runtime_binding,
        $.symbol,
        $.grouped_value,
      ),

    array_value: ($) =>
      seq(
        "[",
        optional($.hspace),
        optional(seq($.value, repeat(seq(",", optional($.hspace), $.value)))),
        optional($.hspace),
        "]",
      ),

    condition_expression: ($) =>
      prec.right(seq(
        $.expression_part,
        repeat(seq($.hspace, $.expression_part)),
      )),

    // Complete statement markers at the beginning of an indented line are
    // structural; marker-like near-misses remain prose. This mirrors the
    // production parser's recovery boundary without making semantic claims
    // about the prose content. An ordinary hyphen is prose, while `->`
    // remains a divert marker.
    prose_text: ($) =>
      seq(
        choice(
          $.markup_tag,
          $.interpolation,
          $.escaped_brace,
          $.prose_marker_text,
          $.prose_start,
        ),
        repeat(choice($.markup_tag, $.interpolation, $.escaped_brace, $.prose_content)),
      ),

    markup_tag: ($) =>
      prec.dynamic(
        1,
        seq(
          "[",
          optional(token(prec(1, "/"))),
          field("name", $.markup_name),
          "]",
        ),
      ),

    interpolation: ($) =>
      seq(
        "{",
        field("name", $.placeholder),
        "}",
      ),

    // The production text scanner consumes a backslash with an immediately
    // following brace as one escaped literal. Keeping this token ahead of
    // `interpolation` prevents `\{name\}` from becoming a placeholder;
    // leaving ordinary backslashes in their own prose token also preserves
    // the production's left-to-right `\\{name}` edge (the second slash
    // escapes the opening brace, while the closing brace remains unescaped).
    escaped_brace: (_$) => token(prec(1, /\\[{}]/)),

    // Header IDs are deliberately syntax-only. Keep the label and any
    // author-entered suffix available to editor tooling, including while a
    // draft is incomplete or semantically malformed. The parser/compiler
    // owns the stable-anchor policy.
    line_name: ($) => prec(1, seq($.identifier, optional(seq("@", optional($.id_suffix))))),
    choice_name: ($) => prec(1, seq($.identifier, optional(seq("@", optional($.id_suffix))))),
    target: ($) => choice($.end_target, /[^\s\r\n#]+/),
    end_target: (_$) => token(prec(1, "END")),

    block_name: ($) => $.identifier,
    id_suffix: ($) => choice($.stable_id, $.draft_id),
    // `stable_id` is a useful lexical classification for captures, not a
    // validity decision. A draft or malformed suffix remains an id fragment
    // in the tree and is validated by the production parser/compiler.
    stable_id: (_$) => token(prec(2, /[0-9a-f]{20}/)),
    // Keep the same lexical precedence as stable_id so a longer malformed
    // suffix wins as one token instead of leaving a valid-looking prefix and
    // an ERROR node behind.
    draft_id: (_$) => token(prec(2, /[^\s#]+/)),
    function_name: ($) => prec(1, $.identifier),
    metadata_key: ($) => $.identifier,
    type_name: ($) => $.identifier,
    markup_name: (_$) => /[A-Za-z][A-Za-z0-9_-]*/,
    placeholder: ($) => $.identifier,
    symbol: ($) => prec(0, $.identifier),

    // Identifier spelling remains deliberately broad enough for the current
    // Unicode source fixtures. Compiler validation owns the exact XID policy.
    // Quote and numeric/sign prefixes are excluded so scalar tokens cannot be
    // swallowed by the recovery-friendly symbol rule.
    // Rust's regex engine also parses these patterns and requires escaped '['.
    // A raw string preserves that spelling through JavaScript linting.
    identifier: (_$) => new RegExp(String.raw`[^\s@"$=()\[\]{}|,:0-9+-][^\s@"$=()\[\]{}|,:]*`),

    runtime_binding: ($) => seq("$", $.identifier),
    string: (_$) => token(prec(2, /"(?:\\.|[^"\\\r\n])*"/)),
    number: (_$) => token(prec(2, /[+-]?[0-9]+(?:\.[0-9]+)?/)),
    boolean: (_$) => choice("true", "false"),
    operator: (_$) => choice("and", "or", "not"),

    block_default: (_$) => "default",
    requires_key: (_$) => "requires",
    reason_key: (_$) => "reason",
    effect_mode: (_$) => choice("immediate", "deferred", "blocking"),

    // Prefer the exact marker over permissive prose recovery so an indented
    // block header retains its structural node and named fields.
    block_marker: (_$) => token(prec(1, "::")),
    line_marker: (_$) => ">",
    choice_marker: (_$) => "?",
    effect_marker: (_$) => "!",
    divert_marker: (_$) => "->",
    if_marker: (_$) => ":if",
    else_marker: (_$) => ":else",
    match_marker: (_$) => ":match",
    case_marker: (_$) => ":case",
    plural_marker: (_$) => "|",
    comment_marker: (_$) => "#",

    inline_comment: ($) =>
      seq(
        $.hspace,
        field("marker", $.comment_marker),
        $.comment_text,
      ),

    comment_text: (_$) => /[^\r\n]*/,
    // Rust's `char::is_whitespace` is the production marker-boundary
    // contract. Physical CR/LF are handled by `newline`; the remaining
    // Unicode White_Space scalars are enumerated here because the
    // pinned Tree-sitter regex dialect has no Unicode property escapes.
    // A directive marker is only structural when its complete spelling is
    // followed by horizontal whitespace or the end of the physical line.
    // Keep near-misses in prose without relying on unsupported regex
    // look-around: the alternatives consume the first character that makes a
    // marker-like prefix non-structural, while the one-character fallback
    // keeps ordinary colon-led prose available to the editor grammar.
    prose_start: (_$) =>
      choice(
        new RegExp(String.raw`[^\r\n{}\[\]?#>!:|\\]+`),
        /\\/,
        /:/,
        "[",
        "]",
      ),
    // Consume a marker-like near-miss through the end of its physical line so
    // punctuation cannot become stranded as fake markup or interpolation.
    // The production parser owns the whole line as prose; this rule makes no
    // structured-content claim about it.
    prose_marker_text: (_$) =>
      choice(
        new RegExp(`:[^iemc${directiveNonWhitespace}][^\\r\\n]*`),
        new RegExp(`:i(?:[^f${directiveNonWhitespace}]|f[^${directiveNonWhitespace}])[^\\r\\n]*`),
        new RegExp(
          `:e(?:[^l${directiveNonWhitespace}]|l(?:[^s${directiveNonWhitespace}]|s(?:[^e${directiveNonWhitespace}]|e[^${directiveNonWhitespace}])))[^\\r\\n]*`,
        ),
        new RegExp(
          `:m(?:[^a${directiveNonWhitespace}]|a(?:[^t${directiveNonWhitespace}]|t(?:[^c${directiveNonWhitespace}]|c(?:[^h${directiveNonWhitespace}]|h[^${directiveNonWhitespace}]))))[^\\r\\n]*`,
        ),
        new RegExp(
          `:c(?:[^a${directiveNonWhitespace}]|a(?:[^s${directiveNonWhitespace}]|s(?:[^e${directiveNonWhitespace}]|e[^${directiveNonWhitespace}])))[^\\r\\n]*`,
        ),
      ),
    prose_content: (_$) =>
      choice(
        new RegExp(String.raw`[^\r\n{}\[\]\\]+`),
        /\\/,
        "[",
        "]",
        token(prec(1, "/")),
      ),
    indent: (_$) => /[ \t]+/,
    hspace: (_$) => /[ \t]+/,
    newline: (_$) => /\r?\n/,
  },
});
