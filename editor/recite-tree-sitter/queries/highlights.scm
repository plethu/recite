; Recite highlighting is intentionally syntax-only. No capture below asserts
; that an ID is valid/unique, a reference resolves, a call type-checks, a
; markup tag is balanced, or a match is exhaustive.

; Trivia and statement vocabulary.
((comment_marker) @comment)
((comment_text) @comment)
((block_marker) @keyword)
((line_marker) @punctuation.special)
((choice_marker) @punctuation.special)
((effect_marker) @punctuation.special)
((divert_marker) @punctuation.special)
((if_marker) @keyword.conditional)
((else_marker) @keyword.conditional)
((match_marker) @keyword.conditional)
((case_marker) @keyword.conditional)
((plural_marker) @punctuation.special)

; Author-visible identities and targets.
((block_name) @label)
((line_name (identifier) @label))
((choice_name (identifier) @label))
((stable_id) @label)
((target) @variable)

; Dedicated words and header fields.
((block_default) @constant.builtin)
((effect_mode) @keyword)
((requires_key) @keyword.conditional)
((reason_key) @property)
((metadata_key) @property)
((type_name) @type)

; Values, calls, and their delimiters.
((function_name) @function.call)
((string) @string)
((number) @number)
((boolean) @boolean)
((runtime_binding) @variable.builtin)
((symbol) @constant)
((operator) @operator)
(["(" ")" "[" "]" "{" "}"] @punctuation.bracket)
(["," ":" "="] @punctuation.delimiter)

; Localisable prose, markup, and placeholders.
((prose_content) @string.special)
((markup_name) @tag)
((interpolation (placeholder) @variable.parameter))

; The error capture is recovery-only. Semantic correctness remains elsewhere.
((ERROR) @error)
