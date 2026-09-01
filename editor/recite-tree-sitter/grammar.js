// Tree-sitter is an editor-facing syntax layer for Recite.
//
// This grammar deliberately does not model indentation ownership or any
// semantic rule. Rowan, the compiler, and the LSP remain authoritative for
// source recovery, stable IDs, references, schema, conditions, effects,
// markup, and match exhaustiveness.

module.exports = grammar({
  name: "recite",

  // Whitespace and line endings are part of the grammar. Keeping them out of
  // `extras` makes the line-oriented shape useful to editor structural tools
  // without pretending that this grammar owns Recite's indentation semantics.
  extras: ($) => [],

  rules: {
    source_file: ($) => repeat(choice(
      $.blank_line,
      $.comment_line,
      $.block_statement,
      $.line_statement,
      $.choice_statement,
      $.effect_statement,
      $.divert_statement,
      $.if_statement,
      $.else_statement,
      $.match_statement,
      $.case_statement,
      $.plural_line,
      $.prose_line,
    )),

    blank_line: ($) => $.newline,

    comment_line: ($) => seq(
      optional($.indent),
      field("marker", $.comment_marker),
      $.comment_text,
      $.newline,
    ),

    block_statement: ($) => seq(
      field("marker", $.block_marker),
      $.hspace,
      field("name", $.block_name),
      repeat($.block_attribute),
      optional($.inline_comment),
      $.newline,
    ),

    line_statement: ($) => seq(
      optional($.indent),
      field("marker", $.line_marker),
      $.hspace,
      field("name", $.line_name),
      repeat($.header_attribute),
      optional($.inline_comment),
      $.newline,
    ),

    choice_statement: ($) => seq(
      optional($.indent),
      field("marker", $.choice_marker),
      $.hspace,
      field("name", $.choice_name),
      repeat($.choice_attribute),
      optional($.inline_comment),
      $.newline,
    ),

    effect_statement: ($) => seq(
      optional($.indent),
      field("marker", $.effect_marker),
      $.hspace,
      field("mode", $.effect_mode),
      $.hspace,
      field("call", $.call),
      optional($.inline_comment),
      $.newline,
    ),

    divert_statement: ($) => seq(
      optional($.indent),
      field("marker", $.divert_marker),
      $.hspace,
      field("target", $.target),
      optional($.inline_comment),
      $.newline,
    ),

    if_statement: ($) => seq(
      optional($.indent),
      field("marker", $.if_marker),
      optional($.hspace),
      field("condition", $.condition_expression),
      optional($.inline_comment),
      $.newline,
    ),

    else_statement: ($) => seq(
      optional($.indent),
      field("marker", $.else_marker),
      optional($.inline_comment),
      $.newline,
    ),

    match_statement: ($) => seq(
      optional($.indent),
      field("marker", $.match_marker),
      optional($.hspace),
      field("condition", $.condition_expression),
      optional($.inline_comment),
      $.newline,
    ),

    case_statement: ($) => seq(
      optional($.indent),
      field("marker", $.case_marker),
      $.hspace,
      field("variant", $.identifier),
      optional($.inline_comment),
      $.newline,
    ),

    // `|` is syntax-only here. Whether it is the second source form of a
    // plural line is a compiler/parser decision, not a Tree-sitter decision.
    plural_line: ($) => seq(
      optional($.indent),
      field("marker", $.plural_marker),
      optional($.hspace),
      field("text", $.prose_text),
      $.newline,
    ),

    prose_line: ($) => seq(
      $.indent,
      field("text", $.prose_text),
      $.newline,
    ),

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

    call: ($) => seq(
      field("function", $.function_name),
      "(",
      optional($.arguments),
      optional($.hspace),
      ")",
    ),

    arguments: ($) => seq(
      $.argument,
      repeat(seq(",", optional($.hspace), $.argument)),
    ),

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

    // Statement markers at the beginning of an indented line are always
    // structural. This mirrors the production parser's recovery boundary and
    // leaves marker-leading prose for an explicit future syntax decision.
    prose_text: ($) => seq(
      choice($.markup_tag, $.interpolation, $.prose_start),
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

    line_name: ($) => seq($.identifier, "@", $.stable_id),
    choice_name: ($) => seq($.identifier, "@", $.stable_id),
    target: ($) => /[^\s\r\n#]+/,

    block_name: ($) => $.identifier,
    stable_id: ($) => /[0-9a-f]{20}/,
    function_name: ($) => prec(1, $.identifier),
    metadata_key: ($) => $.identifier,
    type_name: ($) => $.identifier,
    markup_name: ($) => /[A-Za-z][A-Za-z0-9_-]*/,
    placeholder: ($) => $.identifier,
    symbol: ($) => prec(0, $.identifier),

    // Identifier spelling remains deliberately broad enough for the current
    // Unicode source fixtures. Compiler validation owns the exact XID policy.
    identifier: ($) => /[^\s@$=()\[\]{}|,:0-9][^\s@$=()\[\]{}|,:]*/,

    runtime_binding: ($) => seq("$", $.identifier),
    // The closing quote is optional so an editor buffer remains highlightable
    // while a literal is being typed; compiler syntax diagnostics own the
    // malformed case.
    string: ($) => /"(?:\\.|[^"\\\r\n])*"?/,
    number: ($) => /[0-9]+(?:\.[0-9]+)?/,
    boolean: ($) => choice("true", "false"),
    operator: ($) => choice("and", "or", "not", "==", "!=", ">=", "<=", ">", "<"),

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
    prose_start: ($) => /[^\r\n{}\[\]?#>!:|\-]+/,
    prose_content: ($) => /[^\r\n{}\[\]]+/,
    indent: ($) => /[ \t]+/,
    hspace: ($) => /[ \t]+/,
    newline: ($) => /\r?\n/,
  },
});
