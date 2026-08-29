# Human-authored English diagnostic explanation resources.
#
# First-party primary and variant presentation resources are checked against
# the core contracts. Producers may continue supplying the deterministic
# compatibility message until their migration lands. This single adapter is
# only for that legacy English compatibility field.
diagnostic-legacy-message = {$message}

# RECITE_PARSE001
diagnostic-parse-001 = expected a Recite statement header or indented prose

# RECITE_PARSE002
diagnostic-parse-002 = statement appears before a block header

# RECITE_PARSE003
diagnostic-parse-003 = block header must include a block id

# RECITE_PARSE005
diagnostic-parse-005 = block id must not be empty

# RECITE_PARSE007
diagnostic-parse-007 = mixed indentation inside statement body

# RECITE_PARSE008
diagnostic-parse-008 = malformed statement header field

# RECITE_PARSE010
diagnostic-parse-010 = divert header must include a target

# RECITE_PARSE011
diagnostic-parse-011 = malformed divert target

# RECITE_PARSE012
diagnostic-parse-012 = { $reason ->
    [missing_mode] malformed effect statement: missing effect mode
    [invalid_mode] malformed effect statement: expected deferred, immediate, or blocking
    [unterminated_string] malformed effect statement: unterminated string literal
    [invalid_float] malformed effect statement: invalid float literal
    [invalid_integer] malformed effect statement: invalid integer literal
    [expected_function_call] malformed effect statement: expected function call
    [expected_function_name_paren] malformed effect statement: expected '(' after function name
    [expected_right_paren] malformed effect statement: expected ')'
    [expected_scalar_argument] malformed effect statement: expected scalar argument
    [unexpected_trailing_tokens] malformed effect statement: unexpected trailing tokens
   *[other] malformed effect statement: invalid syntax
}
diagnostic-parse-012-unexpected-character = malformed effect statement: unexpected character '{$character}'

# RECITE_PARSE013
diagnostic-parse-013 = { $reason ->
    [unterminated_string] malformed condition expression: unterminated string literal
    [invalid_float] malformed condition expression: invalid float literal
    [invalid_integer] malformed condition expression: invalid integer literal
    [expected_function_call] malformed condition expression: expected function call
    [expected_function_name_paren] malformed condition expression: expected '(' after function name
    [expected_right_paren] malformed condition expression: expected ')'
    [expected_scalar_argument] malformed condition expression: expected scalar argument
    [unexpected_trailing_tokens] malformed condition expression: unexpected trailing tokens
   *[other] malformed condition expression: invalid syntax
}
diagnostic-parse-013-unexpected-character = malformed condition expression: unexpected character '{$character}'

# RECITE_PARSE014
diagnostic-parse-014 = case header must include a variant or _

# RECITE_PARSE015
diagnostic-parse-015 = :else must immediately follow a sibling :if body

# RECITE_PARSE016
diagnostic-parse-016 = :case must appear inside a :match body

# RECITE_PARSE017
diagnostic-parse-017 = prose cannot follow nested statements in the same body

# RECITE_PARSE018
diagnostic-parse-018 = old trailing choice if syntax is not valid Recite v1 syntax

# RECITE_PARSE034
diagnostic-parse-034-expected-directive = expected PO directive
diagnostic-parse-034-expected-quoted-string = expected quoted PO string
diagnostic-parse-034-missing-field = entry is missing {$field}
diagnostic-parse-034-duplicate-field = duplicate PO field {$field}
diagnostic-parse-034-quoted-without-field = quoted continuation without a PO field
diagnostic-parse-034-unexpected-trailing-text = unexpected text after quoted PO string
diagnostic-parse-034-unterminated-quoted-string = unterminated quoted PO string
diagnostic-parse-034-unsupported-escape = unsupported PO escape {$escape}
diagnostic-parse-034-invalid-field-order = invalid PO field order: {$value}

# RECITE_ID034-035
diagnostic-id-034 = invalid stable PO context `{$context}`
diagnostic-id-035 = duplicate PO catalogue key: context `{$context}` and msgid `{$source_text}`

# RECITE_VALIDATE042-044
diagnostic-validate-042 = PO placeholder mismatch: {$detail}
diagnostic-validate-043-contiguous-arms = plural entries require contiguous msgstr[N] arms
diagnostic-validate-043-expected-arm = expected msgstr[{$expected}]
diagnostic-validate-043-requires-plural-source = msgstr[N] requires msgid_plural
diagnostic-validate-043-count = header declares {$expected} plural arms but entry has {$actual}
diagnostic-validate-043-invalid-arm = invalid plural arm `{$keyword}`
diagnostic-validate-044-multiple-headers = PO document contains multiple header records
diagnostic-validate-044-missing-colon = header line `{$line}` lacks `:`
diagnostic-validate-044-duplicate-or-empty = duplicate or empty header `{$key}`
diagnostic-validate-044-invalid-plural-forms = Plural-Forms must declare positive nplurals and a plural expression
diagnostic-validate-044-invalid-plural-rule = Plural-Forms rule is unusable: {$detail}
diagnostic-validate-044-plural-header-required = active plural entries require Plural-Forms with nplurals and plural
diagnostic-validate-047 = translation changes attributes for inline markup tag `{$tag}`: expected `{$expected}`, got `{$actual}`
diagnostic-validate-048 = translation introduces inline markup tag `{$tag}` not present in the source value
diagnostic-validate-049 = translation is missing required inline markup tag `{$tag}` from the source value

# Compiler diagnostic primary presentations. These resources are deliberately
# kept beside the parser presentations; their exact arguments come from the
# first-party contracts in recite-core.

# RECITE_ID001-008
diagnostic-id-001 = line header must include a stable line id
diagnostic-id-002 = choice header must include a stable choice id
diagnostic-id-003 = duplicate localisable id `{$id}` on line
diagnostic-id-004 = duplicate localisable id `{$id}` on choice
diagnostic-id-005 = line header has an unfrozen draft source id
diagnostic-id-006 = choice header has an unfrozen draft source id
diagnostic-id-007 = line header has malformed source id `{$id}`
diagnostic-id-008 = choice header has malformed source id `{$id}`
diagnostic-id-001-help = add a stable author-visible ID to the line header
diagnostic-id-002-help = add a stable author-visible ID to the choice header
diagnostic-id-003-related = first localisable ID is here
diagnostic-id-003-help = rename one of the duplicate localisable IDs
diagnostic-id-004-related = first localisable ID is here
diagnostic-id-004-help = rename one of the duplicate localisable IDs
diagnostic-id-005-help = freeze the line ID as `label@20hexanchor`
diagnostic-id-006-help = freeze the choice ID as `label@20hexanchor`
diagnostic-id-007-help = use `label@20hexanchor`; plain unsuffixed IDs are invalid
diagnostic-id-008-help = use `label@20hexanchor`; plain unsuffixed IDs are invalid

# RECITE_VALIDATE005-016
diagnostic-validate-005 = project must declare exactly one default block
diagnostic-validate-005-help = mark one block header with `default`
diagnostic-validate-006 = block `{$block_id}` is another default block
diagnostic-validate-006-related = first default block is here
diagnostic-validate-006-help = keep exactly one block marked `default`
diagnostic-validate-007 = unknown block reference `{$reference}`
diagnostic-validate-008-file = invalid source span for { $owner ->
    [block] block
    [comment] comment
    [line] line
    [line-source-text] line source text
    [choice] choice
    [choice-source-text] choice source text
    [choice-availability-requirement] choice availability requirement
    [condition-expression] condition expression
    [condition-call] condition call
    [condition-function] condition function
    [condition-argument] condition argument
    [choice-availability-reason] choice availability reason
    [choice-availability-reason-id] choice availability reason id
    [choice-availability-reason-arguments] choice availability reason arguments
    [choice-target] choice target
    [divert] divert
    [if-branch] if branch
    [match-branch] match branch
    [match-arm] match arm
    [effect] effect
    [effect-mode] effect mode
    [effect-function] effect function
    [effect-call] effect call
    [effect-argument] effect argument
    [plural-source-text] plural source text
    [metadata-entry] metadata entry
    [metadata-key] metadata key
    [metadata-value] metadata value
   *[other] source span
}: span file does not match source file
diagnostic-validate-008-order = invalid source span for { $owner ->
    [block] block
    [comment] comment
    [line] line
    [line-source-text] line source text
    [choice] choice
    [choice-source-text] choice source text
    [choice-availability-requirement] choice availability requirement
    [condition-expression] condition expression
    [condition-call] condition call
    [condition-function] condition function
    [condition-argument] condition argument
    [choice-availability-reason] choice availability reason
    [choice-availability-reason-id] choice availability reason id
    [choice-availability-reason-arguments] choice availability reason arguments
    [choice-target] choice target
    [divert] divert
    [if-branch] if branch
    [match-branch] match branch
    [match-arm] match arm
    [effect] effect
    [effect-mode] effect mode
    [effect-function] effect function
    [effect-call] effect call
    [effect-argument] effect argument
    [plural-source-text] plural source text
    [metadata-entry] metadata entry
    [metadata-key] metadata key
    [metadata-value] metadata value
   *[other] source span
}: span end precedes span start
diagnostic-validate-009 = duplicate block id `{$block_id}`
diagnostic-validate-009-related = first block ID is here
diagnostic-validate-009-help = rename one of the duplicate block IDs
diagnostic-validate-010 = duplicate source path `{$path}`
diagnostic-validate-010-related = first source file with this path is here
diagnostic-validate-010-help = compile each source path once
diagnostic-validate-011 = compiled block id `{$block_id}` must be globally unique
diagnostic-validate-011-related = first compiled block ID is here
diagnostic-validate-011-help = rename one block or split the runtime lookup contract in a future format version
diagnostic-validate-012 = choice must target a block or END before it can be compiled
diagnostic-validate-012-help = add a choice body divert such as `-> next_block` or `-> END`
diagnostic-validate-013 = line `{$line_id}` contains a nested { $statement_kind ->
    [line] line
    [choice] choice
    [divert] divert
    [if] if
    [match] match
    [effect] effect
    [comment] comment
   *[other] statement
} statement that v0 compiled prompts cannot represent
diagnostic-validate-013-related = line containing the unsupported nested statement is here
diagnostic-validate-013-help = keep only nested choices under prompt lines for v0 compiled assets
diagnostic-validate-014 = choice `{$choice_id}` contains a nested { $statement_kind ->
    [line] line
    [choice] choice
    [divert] divert
    [if] if
    [match] match
    [effect] effect
    [comment] comment
   *[other] statement
} statement that v0 compiled choices cannot represent
diagnostic-validate-014-related = choice containing the unsupported nested statement is here
diagnostic-validate-014-help = keep choice bodies to text and one target divert for v0 compiled assets
diagnostic-validate-015 = choice echo references unknown line id `{$line_id}`
diagnostic-validate-015-help = use an existing line ID, `echo=selected_text`, or `echo=none`
diagnostic-validate-016-condition = condition argument contains a non-finite float value
diagnostic-validate-016-effect = effect argument contains a non-finite float value
diagnostic-validate-016-metadata = metadata value `{$key}` contains a non-finite float value
diagnostic-validate-016-help = use a finite number so MessagePack and inspection JSON stay equivalent

# RECITE_VALIDATE017-025
diagnostic-validate-017 = unknown effect function `{$function}`
diagnostic-validate-017-help = declare the effect in the project schema manifest
diagnostic-validate-018 = effect `{$function}` expects {$expected} { $expected ->
    [1] argument
   *[other] arguments
}, but got {$actual}
diagnostic-validate-018-help = match the effect parameters declared in the project schema manifest
diagnostic-validate-019 = argument {$index} for effect `{$function}` expects {$expected}, but got {$actual}
diagnostic-validate-020 = effect `{$function}` does not support {$mode} mode
diagnostic-validate-020-help = use a mode declared for this effect in the project schema manifest
diagnostic-validate-021 = argument {$index} for effect `{$function}` uses unknown {$expected} value `{$value}`
diagnostic-validate-021-help = use a value exported in the project schema manifest
diagnostic-validate-022 = unknown inline markup tag `{$tag}`
diagnostic-validate-022-help = declare the tag in the project schema manifest or remove the markup
diagnostic-validate-023-bracket = unbalanced inline markup tag `[`: missing closing bracket
diagnostic-validate-023-standalone = unbalanced inline markup tag `{$tag}`: standalone tag does not use a closing tag
diagnostic-validate-023-no-opening = unbalanced inline markup tag `{$tag}`: closing tag has no matching opening tag
diagnostic-validate-023-mismatch = unbalanced inline markup tag `{$tag}`: expected closing tag for `{$expected_tag}` first
diagnostic-validate-023-related = open markup tag is here
diagnostic-validate-023-help = balance inline markup tags in localisable source text
diagnostic-validate-024 = inline markup tag `{$tag}` requires a closing tag
diagnostic-validate-024-help = add `[/{ $tag }]` before the localisable text ends
diagnostic-validate-025 = inline markup tag `{$parent}` cannot contain nested tag `{$child}`
diagnostic-validate-025-related = non-nesting markup tag starts here
diagnostic-validate-025-help = close `[{$parent}]` before opening `[{$child}]`

# RECITE_VALIDATE026-041
diagnostic-validate-026 = unknown metadata key `{$key}`
diagnostic-validate-026-help = declare the metadata key in the project schema manifest
diagnostic-validate-027 = metadata key `{$key}` is not allowed on {$target}
diagnostic-validate-027-help = move the metadata entry to an allowed target or update the project schema manifest
diagnostic-validate-028 = metadata key `{$key}` is not repeatable
diagnostic-validate-028-help = remove the duplicate metadata entry or mark the key repeatable in the schema
diagnostic-validate-029 = metadata key `{$key}` expects {$expected}, but got {$actual}
diagnostic-validate-029-help = use a metadata value matching the project schema manifest
diagnostic-validate-030 = metadata key `{$key}` uses unknown {$expected} value `{$value}`
diagnostic-validate-030-help = use a value exported in the project schema manifest
diagnostic-validate-031 = metadata key `{$key}` uses value `{$value}` outside metadata domain `{$domain}`
diagnostic-validate-031-help = use a symbol value exported in the metadata domain snapshot
diagnostic-validate-032 = metadata key `{$key}` cannot resolve selector `{$selector}` for metadata domain `{$domain}`
diagnostic-validate-032-help = provide the selector context or update the domain missing-context policy
diagnostic-validate-033 = metadata key `{$key}` has ambiguous or non-symbol selector `{$selector}`
diagnostic-validate-033-help = metadata domain selectors require exactly one scalar symbol context value
diagnostic-validate-034 = unknown condition function `{$function}`
diagnostic-validate-034-help = declare the condition in the project schema manifest
diagnostic-validate-035 = condition `{$function}` expects {$expected} { $expected ->
    [1] argument
   *[other] arguments
}, but got {$actual}
diagnostic-validate-035-help = match the condition parameters declared in the project schema manifest
diagnostic-validate-036 = argument {$index} for condition `{$function}` expects {$expected}, but got {$actual}
diagnostic-validate-037 = argument {$index} for condition `{$function}` uses unknown {$expected} value `{$value}`
diagnostic-validate-037-help = use a value exported in the project schema manifest
diagnostic-validate-038-bool = condition `{$function}` returns {$actual}, but bool is required
diagnostic-validate-038-enum = condition `{$function}` returns {$actual}, but enum is required
diagnostic-validate-039 = unknown availability reason `{$reason}`
diagnostic-validate-039-help = declare the availability reason in the project schema manifest
diagnostic-validate-040 = availability reason override `{$reason}` must be parameterless
diagnostic-validate-040-help = v1 reason= overrides cannot bind parameters
diagnostic-validate-041 = availability reason `{$reason}` requires a choice requires=(...) clause

# RECITE_VALIDATE045-046
diagnostic-validate-045-unterminated = invalid interpolation binding: unterminated placeholder
diagnostic-validate-045-unescaped = invalid interpolation binding: unescaped closing brace
diagnostic-validate-045-invalid-name = invalid interpolation binding: invalid placeholder name '{$key}'
diagnostic-validate-045-duplicate = invalid interpolation binding: placeholder `{$key}` is declared more than once
diagnostic-validate-045-unused = invalid interpolation binding: binding `{$key}` is not used in the text
diagnostic-validate-045-unbound = invalid interpolation binding: placeholder `{$key}` has no binding
diagnostic-validate-046-newline = invalid plural line: plural lines must contain exactly one singular and one plural body line
diagnostic-validate-046-missing-count = invalid plural line: plural lines require `bind=(count:int=$value)`
diagnostic-validate-046-count-type = invalid plural line: the `count` binding must have type `int`

# RECITE_FRESH001-003
diagnostic-fresh-001 = compiled asset '{$asset}' is stale for source '{$source}'
diagnostic-fresh-002 = compiled asset '{$asset}' has a stale schema fingerprint
diagnostic-fresh-003 = compiled asset '{$asset}' uses compiler compatibility version {$version}, expected {$expected}

# RECITE_PROJECT001-008
diagnostic-project-001 = malformed project manifest: {$detail}
diagnostic-project-002 = duplicate scene id '{$scene_id}'
diagnostic-project-002-related = first scene with this id
diagnostic-project-003 = scene '{$scene_id}' references missing compiled asset '{$asset}'
diagnostic-project-004 = scene '{$scene_id}' references unknown block '{$block}'
diagnostic-project-005 = scene '{$scene_id}' must declare at least one participant
diagnostic-project-006 = compiled asset '{$asset}' references missing source '{$source}'
diagnostic-project-007 = compiled asset '{$asset}' uses unsupported format version {$version}
diagnostic-project-007-malformed = scene '{$scene_id}' references malformed compiled asset '{$asset}': {$detail}
diagnostic-project-008 = scene '{$scene_id}' references unknown participant '{$participant}'
diagnostic-project-008-compiled-asset = scene '{$scene_id}' participant '{$participant}' is not present in compiled asset '{$asset}'

# RECITE_CONFIG101-116
diagnostic-config-101 = project manifest not found: {$detail}
diagnostic-config-102 = could not read project manifest: {$detail}
diagnostic-config-103 = malformed project manifest: {$detail}
diagnostic-config-104 = unsupported project manifest version: {$detail}
diagnostic-config-105 = invalid project source root: {$detail}
diagnostic-config-106 = invalid project exclusion: {$detail}
diagnostic-config-107 = project source root is missing: {$detail}
diagnostic-config-108 = could not read project source root: {$detail}
diagnostic-config-109 = project source root escapes the project: {$detail}
diagnostic-config-110 = duplicate project source root: {$detail}
diagnostic-config-111 = overlapping project source root: {$detail}
diagnostic-config-112 = could not read project source directory: {$detail}
diagnostic-config-113 = project discovery encountered a non-UTF-8 path: {$detail}
diagnostic-config-114 = project source symlink escapes the project: {$detail}
diagnostic-config-115 = project source is not valid UTF-8: {$detail}
diagnostic-config-116 = project source root is not a directory: {$detail}

# RECITE_FRESH001
diagnostic-fresh-001-meaning = A compiled asset was built from an older version of one or more source files.
diagnostic-fresh-001-cause-001 = The source dialogue changed after the asset was compiled.
diagnostic-fresh-001-remediation-001 = Re-run `recite compile` or `recite watch` for the project.

# RECITE_FRESH002
diagnostic-fresh-002-meaning = A compiled asset was built from an older schema fingerprint.
diagnostic-fresh-002-cause-001 = The schema manifest changed after the asset was compiled.
diagnostic-fresh-002-remediation-001 = Recompile the asset with the current schema manifest.

# RECITE_FRESH003
diagnostic-fresh-003-meaning = A compiled asset was produced with a compiler compatibility version that this Recite version cannot use.
diagnostic-fresh-003-cause-001 = The asset's compiler compatibility version is older or newer than the supported boundary.
diagnostic-fresh-003-remediation-001 = Recompile the source with the current Recite compiler.

# RECITE_ID001
diagnostic-id-001-meaning = A dialogue line is missing its required stable line ID.
diagnostic-id-001-cause-001 = A line was written without an `@id` anchor.
diagnostic-id-001-remediation-001 = Add a stable line ID and keep it unchanged once authored.

# RECITE_ID002
diagnostic-id-002-meaning = A choice is missing its required stable choice ID.
diagnostic-id-002-cause-001 = A choice was written without an `@id` anchor.
diagnostic-id-002-remediation-001 = Add a stable choice ID and keep it unchanged once authored.

# RECITE_ID003
diagnostic-id-003-meaning = Two or more lines use the same stable line ID.
diagnostic-id-003-cause-001 = A line was copied without changing the ID.
diagnostic-id-003-remediation-001 = Give each distinct line a unique stable line ID.

# RECITE_ID004
diagnostic-id-004-meaning = Two or more choices use the same stable choice ID.
diagnostic-id-004-cause-001 = A choice was copied without changing the ID.
diagnostic-id-004-remediation-001 = Give each distinct choice a unique stable choice ID.

# RECITE_ID005
diagnostic-id-005-meaning = A line still uses a draft line ID.
diagnostic-id-005-cause-001 = The line was generated or stubbed and was never assigned a final ID.
diagnostic-id-005-remediation-001 = Replace the draft ID with the final stable line ID before shipping.

# RECITE_ID006
diagnostic-id-006-meaning = A choice still uses a draft choice ID.
diagnostic-id-006-cause-001 = The choice was generated or stubbed and was never assigned a final ID.
diagnostic-id-006-remediation-001 = Replace the draft ID with the final stable choice ID before shipping.

# RECITE_ID007
diagnostic-id-007-meaning = A line ID has an invalid shape.
diagnostic-id-007-cause-001 = The ID contains unsupported characters or does not match the stable ID format.
diagnostic-id-007-remediation-001 = Rename the line ID to the supported stable ID shape.

# RECITE_ID008
diagnostic-id-008-meaning = A choice ID has an invalid shape.
diagnostic-id-008-cause-001 = The ID contains unsupported characters or does not match the stable ID format.
diagnostic-id-008-remediation-001 = Rename the choice ID to the supported stable ID shape.

# RECITE_ID034
diagnostic-id-034-meaning = A PO catalogue stable ID is malformed.
diagnostic-id-034-cause-001 = A PO context or extracted source-ID comment is not a frozen Recite ID.
diagnostic-id-034-remediation-001 = Restore the stable ID and keep its source anchor unchanged.

# RECITE_ID035
diagnostic-id-035-meaning = A PO catalogue contains duplicate active durable keys.
diagnostic-id-035-cause-001 = Two active entries use the same stable context and source text.
diagnostic-id-035-remediation-001 = Keep one active entry per stable catalogue key; retain stale history as fuzzy or obsolete.

# RECITE_PARSE001
diagnostic-parse-001-meaning = The parser found source text that does not match Recite syntax.
diagnostic-parse-001-cause-001 = A statement marker, indentation level, or directive is malformed.
diagnostic-parse-001-remediation-001 = Check the reported span and rewrite the line using the Recite source format.

# RECITE_PARSE002
diagnostic-parse-002-meaning = A statement appears before any block header.
diagnostic-parse-002-cause-001 = The file starts with prose or another statement before the first `:: block` header.
diagnostic-parse-002-remediation-001 = Add the missing block header before the statement or move the statement into an existing block.

# RECITE_PARSE003
diagnostic-parse-003-meaning = A block header is missing its block ID.
diagnostic-parse-003-cause-001 = A `::` header was written without a following identifier.
diagnostic-parse-003-remediation-001 = Add the block ID after `::`.

# RECITE_PARSE005
diagnostic-parse-005-meaning = A block header contains an empty block ID.
diagnostic-parse-005-cause-001 = The header contains only whitespace where the block ID should be.
diagnostic-parse-005-remediation-001 = Replace the empty block ID with a valid block name.

# RECITE_PARSE007
diagnostic-parse-007-meaning = A statement body mixes indentation styles or indentation widths.
diagnostic-parse-007-cause-001 = Indented child lines under the same statement do not align consistently.
diagnostic-parse-007-remediation-001 = Make the nested body use one consistent indentation level.

# RECITE_PARSE008
diagnostic-parse-008-meaning = A statement header field is malformed.
diagnostic-parse-008-cause-001 = A block, choice, metadata, or condition header contains a field the parser cannot read.
diagnostic-parse-008-remediation-001 = Rewrite the reported header field using the supported Recite statement syntax.

# RECITE_PARSE010
diagnostic-parse-010-meaning = A divert header is missing its target.
diagnostic-parse-010-cause-001 = A `->` divert was written without a following block ID, external target, or END.
diagnostic-parse-010-remediation-001 = Add the intended divert target after `->`.

# RECITE_PARSE011
diagnostic-parse-011-meaning = A divert target is malformed.
diagnostic-parse-011-cause-001 = The target after `->` does not match a valid block, external block, or END target.
diagnostic-parse-011-remediation-001 = Rewrite the divert target using Recite's supported target syntax.

# RECITE_PARSE012
diagnostic-parse-012-meaning = An effect statement is malformed.
diagnostic-parse-012-cause-001 = An effect function name, mode, or argument list is incomplete.
diagnostic-parse-012-remediation-001 = Rewrite the effect using the supported effect call syntax.

# RECITE_PARSE013
diagnostic-parse-013-meaning = A condition expression is malformed.
diagnostic-parse-013-cause-001 = A condition call, operator, grouping, or argument list is incomplete.
diagnostic-parse-013-remediation-001 = Rewrite the condition expression with valid Recite condition syntax.

# RECITE_PARSE014
diagnostic-parse-014-meaning = A match case is malformed.
diagnostic-parse-014-cause-001 = A case pattern or case body is incomplete.
diagnostic-parse-014-remediation-001 = Rewrite the case with a supported pattern and body.

# RECITE_PARSE015
diagnostic-parse-015-meaning = An `else` clause appears where no matching `if` can own it.
diagnostic-parse-015-cause-001 = The `else` is incorrectly indented or placed outside its conditional group.
diagnostic-parse-015-remediation-001 = Move the `else` under the intended `if` or remove it.

# RECITE_PARSE016
diagnostic-parse-016-meaning = A `case` clause appears where no matching `match` can own it.
diagnostic-parse-016-cause-001 = The `case` is incorrectly indented or placed outside its match group.
diagnostic-parse-016-remediation-001 = Move the `case` under the intended `match` or remove it.

# RECITE_PARSE017
diagnostic-parse-017-meaning = Prose appears after a nested statement in the same body.
diagnostic-parse-017-cause-001 = A line body mixes nested statements and later prose where the parser cannot preserve ownership.
diagnostic-parse-017-remediation-001 = Move the prose before the nested statements or split it into a separate line body.

# RECITE_PARSE018
diagnostic-parse-018-meaning = A choice has a trailing `if` clause that is not valid Recite syntax.
diagnostic-parse-018-cause-001 = Condition syntax from another dialogue format was used after a choice.
diagnostic-parse-018-remediation-001 = Use Recite's supported `requires` availability clause instead.

# RECITE_PARSE034
diagnostic-parse-034-meaning = A gettext PO record is not structurally valid.
diagnostic-parse-034-cause-001 = A PO directive, quoted string, continuation, or field boundary is malformed.
diagnostic-parse-034-remediation-001 = Fix the reported PO record while preserving its surrounding comments and fields.

# RECITE_PROJECT001
diagnostic-project-001-meaning = The project manifest could not be parsed or does not match the manifest shape.
diagnostic-project-001-cause-001 = `recite.project.toml` is malformed or missing required fields.
diagnostic-project-001-remediation-001 = Fix the manifest TOML and required project fields.

# RECITE_PROJECT002
diagnostic-project-002-meaning = Two scenes in the project manifest use the same scene ID.
diagnostic-project-002-cause-001 = A scene entry was copied without changing its key.
diagnostic-project-002-remediation-001 = Give each scene a unique scene ID.

# RECITE_PROJECT003
diagnostic-project-003-meaning = A scene manifest references a compiled asset that is missing.
diagnostic-project-003-cause-001 = The asset has not been compiled or the manifest path is wrong.
diagnostic-project-003-remediation-001 = Compile the asset or correct the manifest asset path.

# RECITE_PROJECT004
diagnostic-project-004-meaning = A scene manifest start block does not exist in the compiled asset.
diagnostic-project-004-cause-001 = The source block was renamed or the manifest points at the wrong block.
diagnostic-project-004-remediation-001 = Update the manifest start block or recompile the intended source.

# RECITE_PROJECT005
diagnostic-project-005-meaning = A scene manifest is missing required participants.
diagnostic-project-005-cause-001 = The scene omits participant declarations needed by the project contract.
diagnostic-project-005-remediation-001 = Add the required participant entries to the scene manifest.

# RECITE_PROJECT006
diagnostic-project-006-meaning = A scene manifest references a source asset that is missing.
diagnostic-project-006-cause-001 = The source file was moved, deleted, or the manifest path is wrong.
diagnostic-project-006-remediation-001 = Restore the source file or correct the manifest source path.

# RECITE_PROJECT007
diagnostic-project-007-meaning = A referenced compiled asset is malformed, not a Recite asset, or uses an unsupported format.
diagnostic-project-007-cause-001 = The file cannot be decoded as a valid Recite asset because it is malformed, from another format, or from an unsupported version.
diagnostic-project-007-remediation-001 = Recompile the asset from source with the current Recite compiler.

# RECITE_PROJECT008
diagnostic-project-008-meaning = A project participant reference does not match the declared participant contract.
diagnostic-project-008-cause-001 = The scene manifest omits the participant, or the compiled asset and schema participant sets differ.
diagnostic-project-008-remediation-001 = Declare the participant in the manifest and schema, or recompile the asset from the current participant contract.

# RECITE_SCHEMA001
diagnostic-schema-001-meaning = A schema manifest has a malformed shape.
diagnostic-schema-001-cause-001 = The manifest is valid enough to read but does not match the Recite schema model.
diagnostic-schema-001-remediation-001 = Fix the schema field shape reported by the diagnostic.

# RECITE_SCHEMA002
diagnostic-schema-002-meaning = A schema manifest declares an unsupported schema version.
diagnostic-schema-002-cause-001 = The manifest was produced for a newer or incompatible Recite schema version.
diagnostic-schema-002-remediation-001 = Use a supported schema version or regenerate the manifest with this Recite version.

# RECITE_SCHEMA003
diagnostic-schema-003-meaning = A schema manifest defines the same item more than once.
diagnostic-schema-003-cause-001 = A type, speaker, condition, effect, registry, domain, or reason name is duplicated.
diagnostic-schema-003-remediation-001 = Rename or remove the duplicate schema definition.

# RECITE_SCHEMA004
diagnostic-schema-004-meaning = A schema manifest references an unknown or invalid type.
diagnostic-schema-004-cause-001 = A field, parameter, return value, or availability argument names a type that is not defined.
diagnostic-schema-004-remediation-001 = Define the type or update the reference to an existing schema type.

# Schema diagnostic primary presentations. These IDs are intentionally more
# specific than the machine-facing schema codes; the code remains the stable
# machine category while the presentation identifies the semantic case.
diagnostic-schema-001-json-parse = malformed schema manifest: {$detail}
diagnostic-schema-001-toml-parse = malformed schema source: {$detail}
diagnostic-schema-001-toml-decode = malformed schema source: {$detail}
diagnostic-schema-001-source-non-finite = non-finite TOML numbers are not supported
diagnostic-schema-001-source-legacy-binding = TOML availability reason bindings must use {"{"} kind = "binding", name = "..." {"}"}
diagnostic-schema-001-source-tagged-field = availability reason argument '{$name}' contains an unknown tagged field
diagnostic-schema-001-source-generated-field = generated-only field '{$key}' is not accepted in authoritative TOML
diagnostic-schema-001-source-producer-required = a [producer] table with a stable id is required
diagnostic-schema-001-source-producer-id-required = producer id is required
diagnostic-schema-001-source-producer-id-empty = producer id must not be empty
diagnostic-schema-001-source-producer-kind = standalone TOML producer kind must be 'standalone'
diagnostic-schema-001-read = failed to read schema manifest: {$detail}
diagnostic-schema-002-unsupported-version = unsupported schema manifest version {$version}
diagnostic-schema-001-schema-version-type = schema_version must be an integer
diagnostic-schema-001-float-not-representable = {$owner} must be finite and representable as f64
diagnostic-schema-001-producer-export-version = schema_export_version must be greater than zero
diagnostic-schema-001-producer-content-fingerprint-empty-algorithm = manifest content_fingerprint is invalid: FingerprintAlgorithm must not be empty
diagnostic-schema-001-producer-content-fingerprint-blake3-hex-shape = manifest content_fingerprint is invalid: blake3 producer fingerprint must be even-length hex
diagnostic-schema-001-producer-content-fingerprint-blake3-hex-data = manifest content_fingerprint is invalid: blake3 producer fingerprint must be hex
diagnostic-schema-001-producer-content-fingerprint-empty-digest = manifest content_fingerprint is invalid: FingerprintDigest must not be empty
diagnostic-schema-001-producer-content-fingerprint-blake3-digest-length = manifest content_fingerprint is invalid: blake3 fingerprint digest must be 32 bytes, got {$actual}
diagnostic-schema-001-origin-extension = {$owner} origin extension '{$key}' must be namespaced
diagnostic-schema-001-value-origins = {$owner} value_origins must map values to origins
diagnostic-schema-003-producer-fingerprint = {$owner} repeats producer fingerprint '{$kind}:{$id}'
diagnostic-schema-001-provenance-unknown-value = {$owner} provenance key '{$key}' is not a declared value
diagnostic-schema-001-type-kind = type '{$type}' uses unsupported kind '{$kind}'
diagnostic-schema-003-value = {$owner} repeats value '{$value}'
diagnostic-schema-001-metadata-target = metadata '{$metadata}' uses unsupported target '{$target}'
diagnostic-schema-003-metadata-target = metadata '{$metadata}' repeats target '{$target}'
diagnostic-schema-001-metadata-array-type = metadata '{$metadata}' uses projection-only array type '{$type_ref}'
diagnostic-schema-001-metadata-domain-type = metadata '{$metadata}' uses a metadata domain but has non-symbol type '{$type_ref}'
diagnostic-schema-004-invalid-condition-return = condition '{$condition}' has invalid return type '{$return_type}'
diagnostic-schema-001-effect-mode = effect '{$effect}' uses unsupported mode '{$mode}'
diagnostic-schema-003-effect-mode = effect '{$effect}' repeats mode '{$mode}'
diagnostic-schema-003-parameter = {$owner} repeats parameter '{$parameter}'
diagnostic-schema-004-parameter-special-type = {$owner} parameter '{$parameter}' uses projection-only or metadata-only type reference '{$type_ref}'
diagnostic-schema-004-invalid-metadata-type = metadata '{$metadata}' has invalid type reference '{$type_ref}'
diagnostic-schema-004-invalid-parameter-type = parameter '{$parameter}' has invalid type reference '{$type_ref}'
diagnostic-schema-004-invalid-projection-input-type = projector '{$projector}' input '{$input}' has invalid type reference '{$type_ref}'
diagnostic-schema-004-invalid-projection-output-type = projector '{$projector}' output '{$output}' binding '{$binding}' has invalid type reference '{$type_ref}'
diagnostic-schema-004-invalid-query-return-type = projection query function '{$function}' has invalid return type '{$type_ref}'
diagnostic-schema-004-contextual-domain-for-flat = {$owner} references contextual metadata domain '{$domain}', but a flat domain is required
diagnostic-schema-004-unknown-metadata-domain = {$owner} references unknown metadata domain '{$domain}'
diagnostic-schema-004-unknown-enum = {$owner} references unknown enum type '{$name}'
diagnostic-schema-004-unknown-registry = {$owner} references unknown registry '{$name}'
diagnostic-schema-003-duplicate-definition = duplicate {$kind} definition '{$name}'
diagnostic-schema-001-empty-value = {$field} must not be empty
diagnostic-schema-001-invalid-name = {$field} must be an identifier-like schema name
diagnostic-schema-001-availability-non-bool-mapping = condition '{$condition}' availability_reason mapping is only allowed on bool-returning conditions
diagnostic-schema-004-unknown-availability-reason = condition '{$condition}' availability_reason references unknown reason '{$reason}'
diagnostic-schema-001-availability-template-unterminated = availability reason '{$reason}' template has invalid placeholder syntax: unterminated placeholder
diagnostic-schema-001-availability-template-invalid-name = availability reason '{$reason}' template has invalid placeholder syntax: invalid placeholder name '{$name}'
diagnostic-schema-001-availability-template-unescaped-closing-brace = availability reason '{$reason}' template has invalid placeholder syntax: unescaped closing brace
diagnostic-schema-001-availability-template-unknown-param = availability reason '{$reason}' template references unknown parameter '{$placeholder}'
diagnostic-schema-001-availability-template-unused-param = availability reason '{$reason}' parameter '{$parameter}' is not used in its template
diagnostic-schema-003-availability-argument = condition '{$condition}' availability_reason repeats argument '{$argument}'
diagnostic-schema-001-availability-unknown-reason-param = condition '{$condition}' availability_reason binds unknown reason parameter '{$argument}'
diagnostic-schema-001-availability-missing-reason-arg = condition '{$condition}' availability_reason is missing argument '{$argument}'
diagnostic-schema-001-availability-tagged-only-toml = tagged availability reason arguments are only supported in TOML
diagnostic-schema-001-availability-tag-missing-kind = availability reason argument tag must contain kind
diagnostic-schema-001-availability-binding-missing-name = availability reason binding must contain name
diagnostic-schema-001-availability-literal-missing-value = availability reason literal must contain value
diagnostic-schema-001-availability-tag-kind = unsupported availability reason argument kind '{$kind}'
diagnostic-schema-001-availability-binding-string-type = condition '{$condition}' availability_reason argument '{$argument}' expects {$expected}, but got string literal
diagnostic-schema-001-availability-binding-int = condition '{$condition}' availability_reason argument '{$argument}' expects int, but got non-integer number
diagnostic-schema-001-availability-binding-literal-type = condition '{$condition}' availability_reason argument '{$argument}' expects {$expected}, but got {$actual} literal
diagnostic-schema-001-availability-binding-unknown-value = condition '{$condition}' availability_reason argument '{$argument}' uses unknown {$expected} value '{$value}'
diagnostic-schema-004-unknown-condition-param = condition '{$condition}' availability_reason references unknown condition parameter '{$condition_param}'
diagnostic-schema-001-availability-binding-type-mismatch = condition '{$condition}' availability_reason argument '{$argument}' expects {$expected}, but condition parameter '{$condition_param}' has {$actual}
diagnostic-schema-001-domain-values = metadata domain '{$domain}' requires values
diagnostic-schema-001-domain-missing-context = metadata domain '{$domain}' requires explicit missing_context in generated JSON
diagnostic-schema-001-domain-selector-required = metadata domain '{$domain}' requires selector
diagnostic-schema-001-domain-selector = metadata domain '{$domain}' uses unsupported selector '{$selector}'
diagnostic-schema-001-domain-context-values = metadata domain '{$domain}' requires values_by_context
diagnostic-schema-003-domain-context = metadata domain '{$domain}' repeats context '{$context}'
diagnostic-schema-001-domain-kind = metadata domain '{$domain}' uses unsupported kind '{$kind}'
diagnostic-schema-001-domain-policy-domain = metadata domain '{$domain}' {$policy} policy must not declare domain
diagnostic-schema-001-domain-fallback-domain = metadata domain '{$domain}' fallback policy requires domain
diagnostic-schema-001-domain-policy = metadata domain '{$domain}' uses unsupported missing_context policy '{$policy}'
diagnostic-schema-001-domain-kind-field = metadata domain '{$domain}' does not allow '{$field}' for kind '{$kind}'
diagnostic-schema-001-flat-value-origins = {$owner} flat value_origins must map values to origins
diagnostic-schema-001-context-origin-name = {$owner} provenance context must be an identifier-like name
diagnostic-schema-001-contextual-value-origins = {$owner} contextual value_origins must map contexts to values
diagnostic-schema-001-context-origins = {$owner} context_origins must map contexts to origins
diagnostic-schema-001-projection-candidate-source = projector '{$projector}' input '{$input}' uses an incompatible candidate id source
diagnostic-schema-001-projection-candidate-no-target = projector '{$projector}' input '{$input}' reads candidate metadata but its selector has no metadata target
diagnostic-schema-001-projection-occurrence-repeat = projector '{$projector}' input '{$input}' uses repeated occurrence '{$occurrence}' for non-repeatable metadata key '{$key}'
diagnostic-schema-001-projection-occurrence-all-type = projector '{$projector}' input '{$input}' uses occurrence 'all' but has non-array type {$type_ref}
diagnostic-schema-001-projection-candidate-type-mismatch = projector '{$projector}' input '{$input}' expects {$expected}, but metadata key '{$key}' has {$actual}
diagnostic-schema-001-projection-occurrence-array = projector '{$projector}' input '{$input}' uses array type {$type_ref} without occurrence 'all'
diagnostic-schema-001-projection-reason-no-selector = projector '{$projector}' input '{$input}' reads an availability reason argument but its selector is not availability_reason
diagnostic-schema-004-projection-reason-arg = projector '{$projector}' input '{$input}' references unknown availability reason argument '{$name}'
diagnostic-schema-001-projection-reason-type = projector '{$projector}' input '{$input}' expects {$expected}, but availability reason argument '{$name}' has {$actual}
diagnostic-schema-001-projection-occurrence = projector '{$projector}' input '{$input}' uses unsupported metadata occurrence '{$name}'
diagnostic-schema-003-projection-field = projector '{$projector}' output '{$output}' repeats field '{$field}'
diagnostic-schema-003-projection-input = projector '{$projector}' repeats input '{$input}'
diagnostic-schema-003-label-template = duplicate presentation label template id '{$template_id}'
diagnostic-schema-003-label-argument = projector '{$projector}' output '{$output}' repeats label argument '{$argument}'
diagnostic-schema-001-label-placeholder-unterminated = projector '{$projector}' output '{$output}' presentation label '{$template_id}' has invalid placeholder syntax: unterminated placeholder
diagnostic-schema-001-label-placeholder-invalid-name = projector '{$projector}' output '{$output}' presentation label '{$template_id}' has invalid placeholder syntax: invalid placeholder name '{$name}'
diagnostic-schema-001-label-placeholder-unescaped-closing-brace = projector '{$projector}' output '{$output}' presentation label '{$template_id}' has invalid placeholder syntax: unescaped closing brace
diagnostic-schema-001-label-unknown-arg = projector '{$projector}' output '{$output}' presentation label '{$template_id}' references unknown argument '{$placeholder}'
diagnostic-schema-001-label-unused-arg = projector '{$projector}' output '{$output}' presentation label '{$template_id}' argument '{$arg}' is not used in its template
diagnostic-schema-001-projection-literal-int = {$owner} expects int, but got non-integer number
diagnostic-schema-001-projection-literal-type = {$owner} expects {$expected}, but got {$actual} literal
diagnostic-schema-001-projection-literal-unknown = {$owner} uses unknown {$expected} value '{$value}'
diagnostic-schema-003-projection-output = projector '{$projector}' repeats output '{$output}'
diagnostic-schema-001-projection-output-target = projector '{$projector}' output '{$output}' uses unsupported target '{$target}'
diagnostic-schema-001-query-max-calls = projection query function '{$function}' max_calls_per_event must be greater than zero
diagnostic-schema-003-projection-query = projector '{$projector}' repeats query '{$query}'
diagnostic-schema-004-unknown-query-function = projector '{$projector}' query '{$query}' references unknown projection query function '{$function}'
diagnostic-schema-001-query-arg-count = projector '{$projector}' query '{$query}' passes {$actual} args to projection query function '{$function}', expected {$expected}
diagnostic-schema-004-unknown-projection-ref = projector '{$projector}' {$owner} references unknown {$ref}
diagnostic-schema-001-projection-ref-type = projector '{$projector}' {$owner} expects {$expected}, but {$ref} has {$actual}
diagnostic-schema-003-required-metadata = projector '{$projector}' repeats required metadata key '{$key}'
diagnostic-schema-004-unknown-projection-reason = projector '{$projector}' references unknown availability reason '{$reason}'
diagnostic-schema-001-projection-selector-target = presentation projector uses unsupported metadata target '{$target}'
diagnostic-schema-004-unknown-metadata-key = projector '{$projector}' references unknown metadata key '{$key}'
diagnostic-schema-001-projection-metadata-target = projector '{$projector}' references metadata key '{$key}' on unsupported target '{$target}'

# RECITE_VALIDATE005
diagnostic-validate-005-meaning = The compiled project has no default block.
diagnostic-validate-005-cause-001 = No block is marked as the default start point.
diagnostic-validate-005-remediation-001 = Mark exactly one block as default or provide an explicit start block where required.

# RECITE_VALIDATE006
diagnostic-validate-006-meaning = The compiled project has more than one default block.
diagnostic-validate-006-cause-001 = Multiple blocks were marked as default.
diagnostic-validate-006-remediation-001 = Keep one default block and remove the extra default markers.

# RECITE_VALIDATE007
diagnostic-validate-007-meaning = A divert or reference points at an unknown block.
diagnostic-validate-007-cause-001 = The target block was renamed, removed, or misspelled.
diagnostic-validate-007-remediation-001 = Update the reference or add the missing block.

# RECITE_VALIDATE008
diagnostic-validate-008-meaning = A compiled source span is invalid.
diagnostic-validate-008-cause-001 = Source map data has an impossible line, column, or range.
diagnostic-validate-008-remediation-001 = Report this as a compiler bug with the source that produced it.

# RECITE_VALIDATE009
diagnostic-validate-009-meaning = Two blocks use the same block ID.
diagnostic-validate-009-cause-001 = A block was copied or renamed to an existing ID.
diagnostic-validate-009-remediation-001 = Give each block a unique ID.

# RECITE_VALIDATE010
diagnostic-validate-010-meaning = The same source path appears more than once in one compile input set.
diagnostic-validate-010-cause-001 = The same file was passed directly and through a directory scan.
diagnostic-validate-010-remediation-001 = Remove the duplicate input path.

# RECITE_VALIDATE011
diagnostic-validate-011-meaning = A compiled block ID is ambiguous.
diagnostic-validate-011-cause-001 = Two source files compile blocks with the same runtime block ID.
diagnostic-validate-011-remediation-001 = Rename one block or split the runtime lookup contract in a future format version.

# RECITE_VALIDATE012
diagnostic-validate-012-meaning = A choice is missing its runtime target.
diagnostic-validate-012-cause-001 = The choice does not identify a block, branch, or end target.
diagnostic-validate-012-remediation-001 = Add the intended choice target.

# RECITE_VALIDATE013
diagnostic-validate-013-meaning = A line contains a nested statement that the current compiled asset format cannot represent.
diagnostic-validate-013-cause-001 = A prompt line contains a nested statement other than a supported choice.
diagnostic-validate-013-remediation-001 = Keep only nested choices under prompt lines for v0 compiled assets.

# RECITE_VALIDATE014
diagnostic-validate-014-meaning = A choice contains a nested statement that the current compiled asset format cannot represent.
diagnostic-validate-014-cause-001 = A choice body contains unsupported nested control flow or statements.
diagnostic-validate-014-remediation-001 = Keep choice bodies to text and one target divert for v0 compiled assets.

# RECITE_VALIDATE015
diagnostic-validate-015-meaning = A choice echo references an unknown line.
diagnostic-validate-015-cause-001 = The echo line ID does not exist in the compiled dialogue.
diagnostic-validate-015-remediation-001 = Use an existing line ID, `echo=selected_text`, or `echo=none`.

# RECITE_VALIDATE016
diagnostic-validate-016-meaning = A numeric value is not finite.
diagnostic-validate-016-cause-001 = A schema, metadata, condition, or effect value produced NaN or infinity.
diagnostic-validate-016-remediation-001 = Replace the value with a finite number.

# RECITE_VALIDATE017
diagnostic-validate-017-meaning = An effect call names an unknown effect function.
diagnostic-validate-017-cause-001 = The effect function is misspelled or missing from the schema.
diagnostic-validate-017-remediation-001 = Declare the effect in the schema or correct the function name.

# RECITE_VALIDATE018
diagnostic-validate-018-meaning = An effect call has the wrong number of arguments.
diagnostic-validate-018-cause-001 = The call does not match the parameter count declared by the schema.
diagnostic-validate-018-remediation-001 = Add, remove, or reorder arguments to match the schema.

# RECITE_VALIDATE019
diagnostic-validate-019-meaning = An effect call argument has the wrong type.
diagnostic-validate-019-cause-001 = A literal argument does not match the effect parameter type.
diagnostic-validate-019-remediation-001 = Change the argument value or update the schema parameter type.

# RECITE_VALIDATE020
diagnostic-validate-020-meaning = An effect call uses an unsupported effect mode.
diagnostic-validate-020-cause-001 = The effect mode is not one of the modes accepted for that schema effect.
diagnostic-validate-020-remediation-001 = Use an effect mode supported by the effect declaration.

# RECITE_VALIDATE021
diagnostic-validate-021-meaning = An effect call argument names a value that the schema does not export.
diagnostic-validate-021-cause-001 = An effect argument refers to an enum, registry, speaker, or other schema value that is unknown.
diagnostic-validate-021-remediation-001 = Use a value exported in the project schema manifest.

# RECITE_VALIDATE022
diagnostic-validate-022-meaning = Inline text uses an unknown markup tag.
diagnostic-validate-022-cause-001 = The tag is misspelled or missing from the schema markup declarations.
diagnostic-validate-022-remediation-001 = Fix the tag name or declare it in the schema.

# RECITE_VALIDATE023
diagnostic-validate-023-meaning = Inline markup tags are not balanced.
diagnostic-validate-023-cause-001 = A closing tag does not match the most recent open tag.
diagnostic-validate-023-remediation-001 = Reorder or rename the tags so each opening tag closes correctly.

# RECITE_VALIDATE024
diagnostic-validate-024-meaning = An inline markup tag is missing its closing tag.
diagnostic-validate-024-cause-001 = A tagged span reaches the end of the line while still open.
diagnostic-validate-024-remediation-001 = Add the missing closing tag or remove the opening tag.

# RECITE_VALIDATE025
diagnostic-validate-025-meaning = Inline markup nesting is invalid.
diagnostic-validate-025-cause-001 = The tag order creates an unsupported overlap or nesting shape.
diagnostic-validate-025-remediation-001 = Rewrite the markup so tags nest cleanly.

# RECITE_VALIDATE026
diagnostic-validate-026-meaning = Metadata uses a key that the schema does not know.
diagnostic-validate-026-cause-001 = The key is misspelled or missing from the schema metadata declarations.
diagnostic-validate-026-remediation-001 = Declare the metadata key in the schema or correct the key name.

# RECITE_VALIDATE027
diagnostic-validate-027-meaning = Metadata appears on a target where that key is not allowed.
diagnostic-validate-027-cause-001 = The key is valid but not for this block, line, choice, or project context.
diagnostic-validate-027-remediation-001 = Move the metadata or update the schema target list.

# RECITE_VALIDATE028
diagnostic-validate-028-meaning = Metadata repeats a key that must be unique for that target.
diagnostic-validate-028-cause-001 = The same metadata key was written more than once where repeats are forbidden.
diagnostic-validate-028-remediation-001 = Remove the duplicate or change the schema if repeated values are intended.

# RECITE_VALIDATE029
diagnostic-validate-029-meaning = A metadata value has the wrong type.
diagnostic-validate-029-cause-001 = The source value does not match the type declared by the schema.
diagnostic-validate-029-remediation-001 = Change the value or update the metadata declaration type.

# RECITE_VALIDATE030
diagnostic-validate-030-meaning = A metadata value is outside the allowed domain.
diagnostic-validate-030-cause-001 = The value is not one of the allowed symbols, registry values, or contextual values.
diagnostic-validate-030-remediation-001 = Use an allowed value or update the schema domain.

# RECITE_VALIDATE031
diagnostic-validate-031-meaning = A metadata value is outside a metadata domain snapshot.
diagnostic-validate-031-cause-001 = The value is not one of the symbols exported by the resolved metadata domain.
diagnostic-validate-031-remediation-001 = Use a symbol exported in the metadata domain snapshot or update the domain.

# RECITE_VALIDATE032
diagnostic-validate-032-meaning = A metadata domain selector is missing required context.
diagnostic-validate-032-cause-001 = The metadata location does not provide the context needed to resolve the selector.
diagnostic-validate-032-remediation-001 = Provide the selector context, move the metadata, or update the missing-context policy.

# RECITE_VALIDATE033
diagnostic-validate-033-meaning = A metadata domain selector has ambiguous or non-symbol context.
diagnostic-validate-033-cause-001 = The selector context resolves to zero, multiple, or non-symbol scalar values.
diagnostic-validate-033-remediation-001 = Change the context so the selector resolves to exactly one scalar symbol value.

# RECITE_VALIDATE034
diagnostic-validate-034-meaning = A condition call names an unknown condition function.
diagnostic-validate-034-cause-001 = The condition function is misspelled or missing from the schema.
diagnostic-validate-034-remediation-001 = Declare the condition in the schema or correct the function name.

# RECITE_VALIDATE035
diagnostic-validate-035-meaning = A condition call has the wrong number of arguments.
diagnostic-validate-035-cause-001 = The call does not match the parameter count declared by the schema.
diagnostic-validate-035-remediation-001 = Add, remove, or reorder arguments to match the schema.

# RECITE_VALIDATE036
diagnostic-validate-036-meaning = A condition call argument has the wrong type.
diagnostic-validate-036-cause-001 = A literal argument does not match the condition parameter type.
diagnostic-validate-036-remediation-001 = Change the argument value or update the schema parameter type.

# RECITE_VALIDATE037
diagnostic-validate-037-meaning = A condition call argument value is not exported by the schema.
diagnostic-validate-037-cause-001 = A condition argument names an enum, registry, speaker, or other schema value that is unknown.
diagnostic-validate-037-remediation-001 = Use a value exported in the project schema manifest.

# RECITE_VALIDATE038
diagnostic-validate-038-meaning = A condition returns a value kind that is not valid in this expression context.
diagnostic-validate-038-cause-001 = A non-boolean condition is used where a boolean result is required, or a boolean condition is used as a match scrutinee.
diagnostic-validate-038-remediation-001 = Use a condition with the return type required by the expression context.

# RECITE_VALIDATE039
diagnostic-validate-039-meaning = A choice availability reason override names an unknown reason.
diagnostic-validate-039-cause-001 = The reason override is misspelled or missing from the schema availability reason declarations.
diagnostic-validate-039-remediation-001 = Declare the availability reason in the project schema manifest or correct the reason name.

# RECITE_VALIDATE040
diagnostic-validate-040-meaning = A choice availability reason override names a parameterized reason.
diagnostic-validate-040-cause-001 = The reason template requires parameters, but direct `reason=` overrides cannot bind them.
diagnostic-validate-040-remediation-001 = Use a parameterless reason override or model the reason through a condition mapping.

# RECITE_VALIDATE041
diagnostic-validate-041-meaning = A choice availability reason appears without a requirement.
diagnostic-validate-041-cause-001 = A choice declares an availability reason but has no `requires=(...)` clause to make it unavailable.
diagnostic-validate-041-remediation-001 = Add the required choice condition or remove the unused reason override.

# RECITE_VALIDATE042
diagnostic-validate-042-meaning = A PO translation does not preserve the source placeholders.
diagnostic-validate-042-cause-001 = A translated value is missing a named interpolation or introduces an extra one.
diagnostic-validate-042-remediation-001 = Keep the same placeholder names in the corresponding PO translation.

# RECITE_VALIDATE043
diagnostic-validate-043-meaning = A PO plural record has invalid translation arms.
diagnostic-validate-043-cause-001 = Plural arms are missing, out of order, duplicated, or inconsistent with the header.
diagnostic-validate-043-remediation-001 = Provide contiguous `msgstr[N]` arms matching the locale's declared plural count.

# RECITE_VALIDATE044
diagnostic-validate-044-meaning = A PO header is malformed.
diagnostic-validate-044-cause-001 = A header line is missing its key/value separator, is duplicated, or has invalid plural metadata.
diagnostic-validate-044-remediation-001 = Correct the header while retaining the rest of the catalogue source.

# RECITE_VALIDATE045
diagnostic-validate-045-meaning = A source interpolation binding is malformed or does not match its text.
diagnostic-validate-045-cause-001 = A placeholder is undeclared, a binding is unused or duplicated, or the text contains invalid brace syntax; all are blocking v1 validation errors.
diagnostic-validate-045-remediation-001 = Declare each placeholder with `bind=(name:type=$value)` and escape literal braces.

# RECITE_VALIDATE046
diagnostic-validate-046-meaning = A plural dialogue line does not meet the source shape required for plural selection.
diagnostic-validate-046-cause-001 = The plural source forms do not have the required two-form body shape.
diagnostic-validate-046-cause-002 = The singular or plural form contains a newline instead of exactly one body line.
diagnostic-validate-046-cause-003 = A plural line has no `bind=(count:int=$value)` binding.
diagnostic-validate-046-cause-004 = The `count` binding uses a type other than `int`.
diagnostic-validate-046-remediation-001 = Provide exactly one singular body line and one immediately following `|` continuation.
diagnostic-validate-046-remediation-002 = Keep the singular and plural forms to one body line each.
diagnostic-validate-046-remediation-003 = Declare the count source with `bind=(count:int=$value)`.
diagnostic-validate-046-remediation-004 = Change the `count` binding type to `int`.

# RECITE_VALIDATE047
diagnostic-validate-047-meaning = A PO translation changes an inline markup tag's attributes.
diagnostic-validate-047-cause-001 = Translated markup must preserve the attributes authored in the source value.
diagnostic-validate-047-remediation-001 = Keep each translated tag's attributes identical to its source tag.

# RECITE_VALIDATE048
diagnostic-validate-048-meaning = A PO translation introduces an inline markup tag that is absent from the source value.
diagnostic-validate-048-cause-001 = Translated markup may not add tag occurrences that the source value did not author.
diagnostic-validate-048-remediation-001 = Remove the introduced tag or add the tag to the source value before extracting translations.

# RECITE_VALIDATE049
diagnostic-validate-049-meaning = A PO translation omits an inline markup tag required by the source value.
diagnostic-validate-049-cause-001 = Translated markup must preserve every source tag occurrence, even when prose is reordered.
diagnostic-validate-049-remediation-001 = Restore the missing tag occurrence in the translated value.
