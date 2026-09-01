// Tree-sitter is an editor-facing syntax layer for Recite.
//
// This grammar deliberately does not model indentation ownership or any
// semantic rule. Rowan, the compiler, and the LSP remain authoritative for
// source recovery, stable IDs, references, schema, conditions, effects,
// markup, and match exhaustiveness.

const directiveUnicodeWhitespace = "\\u000B\\u000C\\u0085\\u00A0\\u1680\\u2000-\\u200A\\u2028\\u2029\\u202F\\u205F\\u3000";
const directiveWhitespace = `\\t ${directiveUnicodeWhitespace}`;
const directiveNonWhitespace = `\\r\\n${directiveWhitespace}`;
const directiveUnicodeHspace = new RegExp(`[${directiveUnicodeWhitespace}]+`);

// Tree-sitter has no EOF token. Ordinary lines require a newline; the final
// source arm below reuses these bodies with an empty terminator so internal
// line separation remains mandatory without duplicating syntax.
const lineEnd = ($, terminator) => terminator ?? seq();
const terminated = ($, terminator, parts) => seq(...parts, lineEnd($, terminator));
const commentLine = ($, terminator) => terminated($, terminator, [
  optional($.indent), field("marker", $.comment_marker), $.comment_text,
]);
const blockStatement = ($, terminator) => terminated($, terminator, [
  field("marker", $.block_marker), $.hspace, field("name", $.block_name),
  repeat($.block_attribute), optional($.inline_comment),
]);
const lineStatement = ($, terminator) => terminated($, terminator, [
  optional($.indent), field("marker", $.line_marker), optional($.hspace),
  field("name", optional($.line_name)), repeat($.header_attribute),
  optional($.inline_comment),
]);
const choiceStatement = ($, terminator) => terminated($, terminator, [
  optional($.indent), field("marker", $.choice_marker), optional($.hspace),
  field("name", optional($.choice_name)), repeat($.choice_attribute),
  optional($.inline_comment),
]);
const effectStatement = ($, terminator) => terminated($, terminator, [
  optional($.indent), field("marker", $.effect_marker), $.hspace,
  field("mode", $.effect_mode), $.hspace, field("call", $.call),
  optional($.inline_comment),
]);
const divertStatement = ($, terminator) => terminated($, terminator, [
  optional($.indent), field("marker", $.divert_marker), $.hspace,
  field("target", $.target), optional($.inline_comment),
]);
const conditionalLine = ($, marker, tail, terminator) => seq(
  optional($.indent), field("marker", marker),
  choice(seq(tail($), lineEnd($, terminator)), lineEnd($, terminator)),
);
const conditionTail = ($) => seq(
  choice($.hspace, alias(directiveUnicodeHspace, $.hspace)),
  optional(field("condition", $.condition_expression)), optional($.inline_comment),
);
const ifLine = ($, terminator) => conditionalLine($, $.if_marker, conditionTail, terminator);
const matchLine = ($, terminator) => conditionalLine($, $.match_marker, conditionTail, terminator);
const elseLine = ($, terminator) => seq(
  optional($.indent), field("marker", $.else_marker),
  choice(
    seq($.inline_comment, lineEnd($, terminator)),
    seq(choice($.hspace, alias(directiveUnicodeHspace, $.hspace)), lineEnd($, terminator)),
    lineEnd($, terminator),
  ),
);
const caseTail = ($) => seq(
  choice($.hspace, alias(directiveUnicodeHspace, $.hspace)),
  optional(field("variant", $.identifier)), optional($.inline_comment),
);
const caseLine = ($, terminator) => conditionalLine($, $.case_marker, caseTail, terminator);
const pluralLine = ($, terminator) => terminated($, terminator, [
  optional($.indent), field("marker", $.plural_marker), optional($.hspace),
  field("text", $.prose_text),
]);
const proseLine = ($, terminator) => terminated($, terminator, [
  $.indent, field("text", $.prose_text),
]);

const sourceLines = ($) => choice(
  $.blank_line, $.comment_line, $.block_statement, $.line_statement,
  $.choice_statement, $.effect_statement, $.divert_statement, $.if_statement,
  $.else_statement, $.match_statement, $.case_statement, $.plural_line,
  $.prose_line,
);
const finalLine = ($) => optional(choice(
  alias($._final_blank_line, $.blank_line),
  alias($._final_comment_line, $.comment_line),
  alias($._final_block_statement, $.block_statement),
  alias($._final_line_statement, $.line_statement),
  alias($._final_choice_statement, $.choice_statement),
  alias($._final_effect_statement, $.effect_statement),
  alias($._final_divert_statement, $.divert_statement),
  alias($._final_if_statement, $.if_statement),
  alias($._final_else_statement, $.else_statement),
  alias($._final_match_statement, $.match_statement),
  alias($._final_case_statement, $.case_statement),
  alias($._final_plural_line, $.plural_line),
  alias($._final_prose_line, $.prose_line),
));

module.exports = grammar({
  name: "recite",

  // Whitespace and line endings are part of the grammar. Keeping them out of
  // `extras` makes the line-oriented shape useful to editor structural tools
  // without pretending that this grammar owns Recite's indentation semantics.
  extras: ($) => [],

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

    block_attribute: ($) => seq(
      $.hspace,
      choice($.block_default, $.metadata_field),
    ),

    header_attribute: ($) => seq(
      $.hspace,
      $.metadata_field,
    ),

    choice_attribute: ($) => seq(
      $.hspace,
      choice($.requires_clause, $.reason_clause, $.metadata_field),
    ),

    requires_clause: ($) => seq(
      field("key", $.requires_key),
      optional($.hspace),
      "=",
      optional($.hspace),
      field("value", $.grouped_value),
    ),

    reason_clause: ($) => seq(
      field("key", $.reason_key),
      optional($.hspace),
      "=",
      optional($.hspace),
      field("value", $.value),
    ),

    metadata_field: ($) => seq(
      field("key", $.metadata_key),
      optional($.hspace),
      "=",
      optional($.hspace),
      field("value", $.value),
    ),

    // A grouped value is intentionally permissive. It covers condition
    // operators (`and`, `or`, `not`) and typed bindings without deciding their
    // meaning. The semantic parser/compiler owns those rules.
    grouped_value: ($) => seq(
      "(",
      optional($.hspace),
      $.expression_part,
      repeat(seq($.hspace, $.expression_part)),
      optional($.hspace),
      ")",
    ),

    expression_part: ($) => choice(
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

    call: ($) => prec.right(seq(
      field("function", $.function_name),
      "(",
      optional($.arguments),
      optional($.hspace),
      // A call may be left open while an author is typing. Because the call
      // can end only at the current physical line, recovery cannot consume
      // the next statement's marker.
      optional(")"),
    )),

    arguments: ($) => prec.right(seq(
      $.argument,
      repeat(seq(",", optional($.hspace), $.argument)),
    )),

    argument: ($) => $.value,

    binding: ($) => seq(
      field("name", $.identifier),
      ":",
      field("type", $.type_name),
      "=",
      field("value", $.value),
    ),

    value: ($) => choice(
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

    array_value: ($) => seq(
      "[",
      optional($.hspace),
      optional(seq($.value, repeat(seq(",", optional($.hspace), $.value)))),
      optional($.hspace),
      "]",
    ),

    condition_expression: ($) => prec.right(seq(
      $.expression_part,
      repeat(seq($.hspace, $.expression_part)),
    )),

    // Complete statement markers at the beginning of an indented line are
    // structural; marker-like near-misses remain prose. This mirrors the
    // production parser's recovery boundary without making semantic claims
    // about the prose content. An ordinary hyphen is prose, while `->`
    // remains a divert marker.
    prose_text: ($) => seq(
      choice($.markup_tag, $.interpolation, $.prose_marker_text, $.prose_start),
      repeat(choice($.markup_tag, $.interpolation, $.prose_content)),
    ),

    markup_tag: ($) => seq(
      "[",
      optional("/"),
      field("name", $.markup_name),
      "]",
    ),

    interpolation: ($) => seq(
      "{",
      field("name", $.placeholder),
      "}",
    ),

    // Header IDs are deliberately syntax-only. Keep the label and any
    // author-entered suffix available to editor tooling, including while a
    // draft is incomplete or semantically malformed. The parser/compiler
    // owns the stable-anchor policy.
    line_name: ($) => prec(1, seq($.identifier, optional(seq("@", optional($.id_suffix))))),
    choice_name: ($) => prec(1, seq($.identifier, optional(seq("@", optional($.id_suffix))))),
    target: ($) => choice($.end_target, /[^\s\r\n#]+/),
    end_target: ($) => token(prec(1, "END")),

    block_name: ($) => $.identifier,
    id_suffix: ($) => choice($.stable_id, $.draft_id),
    // `stable_id` is a useful lexical classification for captures, not a
    // validity decision. A draft or malformed suffix remains an id fragment
    // in the tree and is validated by the production parser/compiler.
    stable_id: ($) => token(prec(2, /[0-9a-f]{20}/)),
    // Keep the same lexical precedence as stable_id so a longer malformed
    // suffix wins as one token instead of leaving a valid-looking prefix and
    // an ERROR node behind.
    draft_id: ($) => token(prec(2, /[^\s#]+/)),
    function_name: ($) => prec(1, $.identifier),
    metadata_key: ($) => $.identifier,
    type_name: ($) => $.identifier,
    markup_name: ($) => /[A-Za-z][A-Za-z0-9_-]*/,
    placeholder: ($) => $.identifier,
    symbol: ($) => prec(0, $.identifier),

    // Identifier spelling remains deliberately broad enough for the current
    // Unicode source fixtures. Compiler validation owns the exact XID policy.
    // Quote and numeric/sign prefixes are excluded so scalar tokens cannot be
    // swallowed by the recovery-friendly symbol rule.
    identifier: ($) => /[^\s@"$=()\[\]{}|,:0-9+-][^\s@"$=()\[\]{}|,:]*/,

    runtime_binding: ($) => seq("$", $.identifier),
    string: ($) => token(prec(2, /"(?:\\.|[^"\\\r\n])*"/)),
    number: ($) => token(prec(2, /[+-]?[0-9]+(?:\.[0-9]+)?/)),
    boolean: ($) => choice("true", "false"),
    operator: ($) => choice("and", "or", "not"),

    block_default: ($) => "default",
    requires_key: ($) => "requires",
    reason_key: ($) => "reason",
    effect_mode: ($) => choice("immediate", "deferred", "blocking"),

    block_marker: ($) => "::",
    line_marker: ($) => ">",
    choice_marker: ($) => "?",
    effect_marker: ($) => "!",
    divert_marker: ($) => "->",
    if_marker: ($) => ":if",
    else_marker: ($) => ":else",
    match_marker: ($) => ":match",
    case_marker: ($) => ":case",
    plural_marker: ($) => "|",
    comment_marker: ($) => "#",

    inline_comment: ($) => seq(
      $.hspace,
      field("marker", $.comment_marker),
      $.comment_text,
    ),

    comment_text: ($) => /[^\r\n]*/,
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
    prose_start: ($) => choice(
      /[^\r\n{}\[\]?#>!:|]+/,
      /:/,
    ),
    // Consume a marker-like near-miss through the end of its physical line so
    // punctuation cannot become stranded as fake markup or interpolation.
    // The production parser owns the whole line as prose; this rule makes no
    // structured-content claim about it.
    prose_marker_text: ($) => choice(
      new RegExp(`:[^iemc${directiveNonWhitespace}][^\\r\\n]*`),
      new RegExp(`:i(?:[^f${directiveNonWhitespace}]|f[^${directiveNonWhitespace}])[^\\r\\n]*`),
      new RegExp(`:e(?:[^l${directiveNonWhitespace}]|l(?:[^s${directiveNonWhitespace}]|s(?:[^e${directiveNonWhitespace}]|e[^${directiveNonWhitespace}])))[^\\r\\n]*`),
      new RegExp(`:m(?:[^a${directiveNonWhitespace}]|a(?:[^t${directiveNonWhitespace}]|t(?:[^c${directiveNonWhitespace}]|c(?:[^h${directiveNonWhitespace}]|h[^${directiveNonWhitespace}]))))[^\\r\\n]*`),
      new RegExp(`:c(?:[^a${directiveNonWhitespace}]|a(?:[^s${directiveNonWhitespace}]|s(?:[^e${directiveNonWhitespace}]|e[^${directiveNonWhitespace}])))[^\\r\\n]*`),
    ),
    prose_content: ($) => /[^\r\n{}\[\]]+/,
    indent: ($) => /[ \t]+/,
    hspace: ($) => /[ \t]+/,
    newline: ($) => /\r?\n/,
  },
});
