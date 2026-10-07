// Shared terminated and final-line shapes for the syntax-only grammar.
const directiveUnicodeWhitespace =
  "\\u000B\\u000C\\u0085\\u00A0\\u1680\\u2000-\\u200A\\u2028\\u2029\\u202F\\u205F\\u3000";
const directiveWhitespace = `\\t ${directiveUnicodeWhitespace}`;
const directiveNonWhitespace = `\\r\\n${directiveWhitespace}`;
const directiveUnicodeHspace = new RegExp(`[${directiveUnicodeWhitespace}]+`);

// Tree-sitter has no EOF token. Ordinary lines require a newline; the final
// source arm below reuses these bodies with an empty terminator so internal
// line separation remains mandatory without duplicating syntax.
const lineEnd = ($, terminator) => terminator ?? seq();
const terminated = ($, terminator, parts) => seq(...parts, lineEnd($, terminator));
const commentLine = ($, terminator) =>
  terminated($, terminator, [
    optional($.indent),
    field("marker", $.comment_marker),
    $.comment_text,
  ]);
const blockStatement = ($, terminator) =>
  terminated($, terminator, [
    optional($.indent),
    field("marker", $.block_marker),
    optional($.hspace),
    field("name", $.block_name),
    repeat($.block_attribute),
    optional($.inline_comment),
  ]);
const lineStatement = ($, terminator) =>
  terminated($, terminator, [
    optional($.indent),
    field("marker", $.line_marker),
    optional($.hspace),
    field("name", optional($.line_name)),
    repeat($.header_attribute),
    optional($.inline_comment),
  ]);
const choiceStatement = ($, terminator) =>
  terminated($, terminator, [
    optional($.indent),
    field("marker", $.choice_marker),
    optional($.hspace),
    field("name", optional($.choice_name)),
    repeat($.choice_attribute),
    optional($.inline_comment),
  ]);
const effectStatement = ($, terminator) =>
  terminated($, terminator, [
    optional($.indent),
    field("marker", $.effect_marker),
    optional($.hspace),
    field("mode", $.effect_mode),
    $.hspace,
    field("call", $.call),
    optional($.inline_comment),
  ]);
const divertStatement = ($, terminator) =>
  terminated($, terminator, [
    optional($.indent),
    field("marker", $.divert_marker),
    optional($.hspace),
    field("target", $.target),
    optional($.inline_comment),
  ]);
const conditionalLine = ($, marker, tail, terminator) =>
  seq(
    optional($.indent),
    field("marker", marker),
    choice(seq(tail($), lineEnd($, terminator)), lineEnd($, terminator)),
  );
const conditionTail = ($) =>
  seq(
    choice($.hspace, alias(directiveUnicodeHspace, $.hspace)),
    optional(field("condition", $.condition_expression)),
    optional($.inline_comment),
  );
const ifLine = ($, terminator) => conditionalLine($, $.if_marker, conditionTail, terminator);
const matchLine = ($, terminator) => conditionalLine($, $.match_marker, conditionTail, terminator);
const elseLine = ($, terminator) =>
  seq(
    optional($.indent),
    field("marker", $.else_marker),
    choice(
      seq($.inline_comment, lineEnd($, terminator)),
      seq(choice($.hspace, alias(directiveUnicodeHspace, $.hspace)), lineEnd($, terminator)),
      lineEnd($, terminator),
    ),
  );
const caseTail = ($) =>
  seq(
    choice($.hspace, alias(directiveUnicodeHspace, $.hspace)),
    optional(field("variant", $.identifier)),
    optional($.inline_comment),
  );
const caseLine = ($, terminator) => conditionalLine($, $.case_marker, caseTail, terminator);
const pluralLine = ($, terminator) =>
  terminated($, terminator, [
    optional($.indent),
    field("marker", $.plural_marker),
    optional($.hspace),
    field("text", $.prose_text),
  ]);
const proseLine = ($, terminator) =>
  terminated($, terminator, [
    $.indent,
    field("text", $.prose_text),
  ]);

const sourceLines = ($) =>
  choice(
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
  );
const finalLine = ($) =>
  optional(choice(
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

module.exports = {
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
};
