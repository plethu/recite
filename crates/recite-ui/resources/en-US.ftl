cli-help-about = Recite dialogue compiler and validation CLI.
cli-help-usage-heading = Usage:
cli-help-commands-heading = Commands
cli-help-arguments-heading = Arguments
cli-help-options-heading = Options
cli-help-command-validate = Validate dialogue source without writing compiled output
cli-help-command-compile = Compile dialogue source to a MessagePack .recitec asset
cli-help-command-extract = Extract gettext POT entries
cli-help-command-check-ids = Report stable line and choice ID diagnostics
cli-help-command-check-markup = Validate inline markup, optionally against a schema manifest
cli-help-command-check-metadata = Validate metadata against a schema manifest
cli-help-command-validate-project = Validate recite.project.toml and referenced compiled assets
cli-help-command-check-fresh = Check whether project compiled assets are fresh
cli-help-command-inspect-schema = Inspect standalone TOML or generated schema JSON as deterministic machine-readable JSON
cli-help-command-explain = Explain a stable diagnostic code
cli-help-command-watch = Watch project inputs and rebuild manifest assets
cli-help-command-run = Run a compiled asset headlessly with fixture data
cli-help-command-trace = Emit deterministic JSON for a headless fixture run
cli-help-command-play = Play a compiled asset interactively
cli-help-command-bench = Generate benchmark scale reports for fixtures or a project root
cli-help-arg-paths = One or more .recite files, or directories containing .recite files
cli-help-arg-schema = Generated schema manifest JSON
cli-help-arg-schema-inspection = Standalone Recite schema TOML or generated schema manifest JSON
cli-help-arg-project-root = Project root containing recite.project.toml
cli-help-arg-diagnostic-code = Stable diagnostic code, such as RECITE_PARSE001
cli-help-arg-output-compile = Write MessagePack compiled asset bytes to this path
cli-help-arg-output-extract = Write POT output to this path instead of stdout
cli-help-arg-asset-run = MessagePack .recitec asset to run
cli-help-arg-asset-play = MessagePack .recitec asset to play
cli-help-arg-block = Block ID to start from
cli-help-arg-fixture = TOML fixture with conditions, choices, and effect options
cli-help-arg-ui = Interactive UI mode
cli-help-arg-keymap = TUI keymap. Overrides [ui].keymap in the user config file
cli-help-arg-dialogue-locale = Dialogue content locale to preview through the runtime locale provider
cli-help-arg-dialogue-catalog = Dialogue gettext catalog mapping in LOCALE=PATH form. Repeatable
cli-help-arg-bench-scale = Synthetic fixture scale; repeat to select multiple scales
cli-help-arg-bench-group = Benchmark group; repeat to select multiple groups
cli-help-arg-bench-format = Output format for benchmark reports
cli-help-arg-bench-output = Write the benchmark report to this path
cli-help-arg-bench-baseline = Benchmark JSON report to compare against
cli-help-arg-bench-samples = Number of samples per benchmark
cli-help-arg-help = Print help
cli-help-arg-version = Print version

explain-code = Code: {$code}
explain-category = Category: {$category}
explain-meaning = Meaning: {$meaning}
explain-common-causes = Common causes:
explain-how-to-fix = How to fix:
explain-list-item = - {$item}

watch-building = watch: building {$path}
watch-waiting-for-changes = watch: waiting for changes
watch-rebuilding = watch: rebuilding
watch-build-succeeded = watch: build succeeded ({$count} assets)
watch-build-duration-microseconds = watch: build completed in {$duration} µs
watch-build-duration-milliseconds = watch: build completed in {$duration} ms
watch-build-failed-waiting = watch: build failed; waiting for changes
watch-build-failed = watch: build failed: {$error}
watch-build-failed-partial = watch: build {$status}: partial publication; failed target {$failed}; recovery targets: {$recovery}{$records}
watch-build-failed-indeterminate = watch: build {$status}: publication state is indeterminate; recovery targets: {$recovery}{$records}
watch-build-failed-refused = watch: build {$status}: publication refused: {$reason}{$records}
watch-build-failed-not-attempted = watch: build {$status}: publication not attempted: {$reason}{$records}
watch-build-failed-published = watch: build {$status}: publication unexpectedly reported success{$records}
watch-build-failed-unsupported = watch: build {$status}: publication returned an unsupported outcome{$records}
watch-build-failed-partial-with-failure = watch: build {$status}: partial publication; failed target {$failed}; recovery targets: {$recovery}; {$failure}{$records}
watch-build-failed-indeterminate-with-failure = watch: build {$status}: publication state is indeterminate; recovery targets: {$recovery}; {$failure}{$records}
watch-build-failed-refused-with-failure = watch: build {$status}: publication refused: {$reason}; {$failure}{$records}
watch-build-failed-not-attempted-with-failure = watch: build {$status}: publication not attempted: {$reason}; {$failure}{$records}
watch-build-failed-published-with-failure = watch: build {$status}: publication unexpectedly reported success; {$failure}{$records}
watch-build-failed-unsupported-with-failure = watch: build {$status}: publication returned an unsupported outcome; {$failure}{$records}
watch-build-status-succeeded = succeeded
watch-build-status-failed = failed
watch-build-status-stale = stale
watch-build-status-cancelled = cancelled
watch-build-status-superseded = superseded
watch-build-status-unknown = unknown
watch-build-recovery-targets-empty = <none>
watch-build-recovery-targets-list = {$target}
watch-build-recovery-required = watch: build published {$count} assets; recovery required{$records}
watch-build-recovery-notice = watch: recovery required{$records}
watch-build-recovery-summary = { $count ->
    [0] {$items}
   *[other] ; recovery markers: {$items}
}
watch-build-recovery-record = {$marker}: {$reason}{$detail}
watch-build-recovery-reason-stage-cleanup = stage cleanup failed
watch-build-recovery-reason-publication-indeterminate = publication outcome is indeterminate
watch-build-recovery-reason-publication-uncommitted = publication did not commit this target
watch-build-recovery-reason-unknown = recovery reason unavailable
watch-build-recovery-detail-io = ; I/O cause: {$kind} ({$raw_os_error}) {$message}
watch-build-recovery-io-already-exists = already exists
watch-build-recovery-io-invalid-input = invalid input
watch-build-recovery-io-not-found = not found
watch-build-recovery-io-permission-denied = permission denied
watch-build-recovery-io-other = other I/O failure
watch-build-failure-check-request-mismatch = build check request mismatch
watch-build-failure-check-freshness-mismatch = build check freshness mismatch
watch-build-failure-check-unknown = build check failed for an unknown reason
watch-build-failure-diagnostics = engine returned diagnostics
watch-build-failure-unknown = build failed for an unknown reason
watch-build-failure-engine-invalid-output = engine failure: invalid output
watch-build-failure-engine-host = engine failure: host failure
watch-build-failure-engine-unknown = engine failure: unknown failure
watch-build-failure-duplicate-target = duplicate build target {$target}
watch-build-failure-preparation = could not prepare {$target}: {$reason}
watch-build-failure-reason-rejected = target was rejected
watch-build-failure-reason-storage = storage failure
watch-build-failure-reason-unknown = unknown preparation failure
watch-build-failure-invalid-published-partition = invalid published target partition
watch-build-failure-invalid-partial-partition = invalid partial target partition
watch-build-failure-invalid-recovery-target = invalid recovery target
watch-build-failure-invalid-not-committed = publication did not commit the prepared batch
watch-build-failure-invalid-unknown = publication returned an unknown invalid outcome
watch-build-failure-refusal-stale-build-generation = stale build generation
watch-build-failure-refusal-stale-snapshot-generation = stale snapshot generation
watch-build-failure-refusal-stale-fingerprints = stale build fingerprints
watch-build-failure-refusal-request-identity-mismatch = request identity mismatch
watch-build-failure-refusal-unknown = publication was refused for an unknown reason
watch-build-failure-not-attempted-build-failed = build failed
watch-build-failure-not-attempted-cancelled = build was cancelled
watch-build-failure-not-attempted-superseded = build was superseded
watch-build-failure-not-attempted-stale = build was stale
watch-build-failure-not-attempted-no-candidates = build produced no candidates
watch-build-failure-not-attempted-preparation-failed = build preparation failed
watch-build-failure-not-attempted-invalid-outcome = publication outcome was invalid
watch-build-failure-not-attempted-unknown = publication was not attempted for an unknown reason
watch-event-error = watch: watcher event error: {$error}

play-tui-starting = starting recite play TUI; use --ui plain for line-oriented output

play-start = play asset={$asset} block={$block}
play-line = line {$id}: {$text}
play-prompt-line = prompt {$id}: {$text}
play-prompt = prompt
play-choice-row = {"  "}[{$index}] {$id}: {$text}{ $available ->
    [true] {""}
   *[false]  (unavailable)
}
play-choice-prompt = choice>
play-condition-prompt = condition {$query} [y/n]>
play-condition-result = condition {$query} = {$result}
play-selected-choice = selected choice {$id}
play-effect = effect {$mode} id={$id} function={$function} args={$args}
play-ack-prompt = ack {$id} with Enter or `ack`>
play-ack-completed = acknowledged effect {$id} completed
play-end = end
play-deferred-effects = deferred effects:
play-deferred-effect-row = {"  "}{$function} {$args}
play-invalid-input = invalid input: {$message}

play-error-enter-y-or-n = enter y or n
play-error-enter-enum-variant = enter an enum variant
play-error-press-enter-or-ack = press Enter or type ack
play-error-empty-choice = choice selection cannot be empty
play-error-choice-index-out-of-range = choice index {$index} is out of range; enter 1-{$count} or a choice ID
play-error-choice-id-invalid = invalid choice ID `{$id}`: {$error}
play-error-choice-id-unavailable = choice ID `{$id}` is not available here
play-error-choice-unavailable = choice `{$id}` is unavailable
play-error-choice-unavailable-reason = choice `{$id}` is unavailable: {$reason}
run-effect = effect {$mode} {$function} {$args}

tui-ready = ready
tui-finished = finished
tui-command = command:
tui-command-with-value = command: :{$command}
tui-unknown-command = unknown command: :{$command}
tui-choice-input-prefix = choice id/index>
tui-choice-input = choice id/index> {$input}
tui-enum-variant-input = enum variant> {$input}
tui-condition-yes-row = yes
tui-condition-no-row = no
tui-condition-yes-shortcut-row = (y)es
tui-condition-no-shortcut-row = (n)o
tui-enum-condition-hint = Type an enum variant and press Enter.
tui-ack-enter-hint = Press Enter to acknowledge

tui-header-title = recite play
tui-header-asset = asset
tui-header-block = block
tui-waiting = Waiting for the next runtime event...
tui-metadata-mode = mode
tui-metadata-runtime-effect-id = runtime effect ID
tui-metadata-function = function
tui-metadata-args = args
tui-input-answer = answer
tui-input-enum-variant = variant
tui-input-ack = ack
tui-input-choice = choice
tui-choice-unavailable =  unavailable
tui-choice-unavailable-reason =  unavailable: {$reason}
tui-deferred-queue-title = Deferred Queue
tui-deferred-queue-scheduled = scheduled
tui-deferred-queue-ready-at-end = ready at end

tui-transcript-line = line
tui-transcript-prompt = prompt
tui-transcript-choice = choice
tui-transcript-condition = condition
tui-transcript-effect = effect
tui-transcript-ack = ack
tui-transcript-deferred = deferred
tui-transcript-end = end
tui-transcript-completed = completed
tui-transcript-deferred-effects = deferred effects
tui-transcript-effect-text = {$mode} {$function} {$args}
tui-transcript-deferred-effect-text = {$function} {$args}

tui-help-title = Help
tui-help-key-heading = Key
tui-help-action-heading = Action
tui-help-description-heading = Description
tui-help-action-close = close
tui-help-action-quit = quit
tui-help-action-move = move
tui-help-action-submit = submit
tui-help-action-input = type
tui-help-action-shortcut = shortcut
tui-help-action-command = command
tui-help-action-help = help
tui-help-action-queue = queue
tui-help-description-close = close this help overlay
tui-help-description-open-help = open this help overlay
tui-help-description-quit = quit the current play session
tui-help-description-interrupt = interrupt the current play session
tui-help-description-move-choice = move the highlighted choice
tui-help-description-submit-choice = select the highlighted choice or typed ID/index
tui-help-description-input-choice = type a choice ID or index
tui-help-description-move-condition = move between yes and no
tui-help-description-shortcut-condition = select the matching answer
tui-help-description-submit-condition = submit the highlighted condition answer
tui-help-description-input-enum-condition = type the enum variant returned by this condition
tui-help-description-submit-enum-condition = submit the typed enum variant
tui-help-description-submit-effect = acknowledge the blocking effect
tui-help-description-finished = leave the finished play screen
tui-help-description-command = enter command mode
tui-help-description-queue = expand or collapse deferred effect queue
tui-footer-command = Enter runs command | Esc cancels

cli-error-play-eof = reached EOF while reading {$field}
cli-error-play-invalid-input = invalid play input: {$message}
cli-error-play-interrupted = play interrupted
cli-error-play-tui-requires-terminal = recite play --ui tui requires interactive stdin and stdout; use --ui plain for pipes, CI, or accessibility tools
cli-error-ui-config-read = failed to read UI config {$path}: {$source}
cli-error-ui-config-toml = failed to parse UI config {$path}: {$source}
cli-error-ui-locale-invalid = failed to parse UI config {$path}: invalid [ui].locale `{$locale}`; expected a BCP-47 locale such as "en-US" or "system"
cli-error-dialogue-catalog-conflict = dialogue catalog {$path} has conflicting translations for locale `{$locale}`, context `{$context}`, source text `{$source_text}`
cli-error-dialogue-catalog-plural-forms-conflict = dialogue catalog {$path} has conflicting Plural-Forms headers for locale `{$locale}` (existing `{$existing}`, provided `{$provided}`)
cli-error-dialogue-catalog-malformed = failed to parse dialogue catalog {$path} at line {$line}: {$reason}
cli-error-dialogue-catalog-missing-locale = dialogue catalogs require a dialogue locale; pass --dialogue-locale for play or set [dialogue].locale in the fixture
cli-error-dialogue-catalog-spec-invalid = invalid dialogue catalog `{$spec}`; expected LOCALE=PATH
cli-error-dialogue-locale-invalid = invalid dialogue locale in {$field}: `{$locale}`; expected a BCP-47 locale such as "en-US"
cli-error-generic = {$message}
cli-error-diagnostic-rendering = failed to render diagnostic: {$source}
cli-error-diagnostic-code-malformed = malformed diagnostic code `{$code}`: expected an uppercase namespaced code such as RECITE_PARSE001{ $has_suggestion ->
    [true] ; did you mean `{$suggestion}`?
   *[false] {$suggestion}
}
cli-error-diagnostic-code-unknown = unknown diagnostic code `{$code}`{ $has_suggestion ->
    [true] ; did you mean `{$suggestion}`?
   *[false] {$suggestion}
}
cli-error-ui-catalog = failed to load UI text catalog: {$source}
cli-error-bench = {$message}
cli-error-benchmark = {$message}
cli-error-dialogue-catalog-reason-expected-directive = expected msgctxt, msgid, or msgstr
cli-error-dialogue-catalog-reason-expected-quoted-string = expected quoted gettext string
cli-error-dialogue-catalog-reason-missing-context = entry is missing msgctxt
cli-error-dialogue-catalog-reason-missing-id = entry is missing msgid
cli-error-dialogue-catalog-reason-missing-translation = entry is missing msgstr
cli-error-dialogue-catalog-reason-invalid-header = invalid PO header: {$detail}
cli-error-dialogue-catalog-reason-invalid-plural-rule = invalid PO Plural-Forms rule: {$detail}
cli-error-dialogue-catalog-reason-invalid-stable-id = invalid stable PO context `{$value}`
cli-error-dialogue-catalog-reason-duplicate-field = duplicate PO field {$field}
cli-error-dialogue-catalog-reason-duplicate-entry = duplicate PO catalogue entry `{$key}`
cli-error-dialogue-catalog-reason-invalid-field-order = {$detail}
cli-error-dialogue-catalog-reason-placeholder-mismatch = {$detail}
cli-error-dialogue-catalog-reason-plural-entries-unsupported = plural entries are not supported
cli-error-dialogue-catalog-reason-quoted-continuation-without-field = quoted continuation without msgctxt, msgid, or msgstr
cli-error-dialogue-catalog-reason-unexpected-text-after-quoted-string = unexpected text after quoted string
cli-error-dialogue-catalog-reason-unterminated-quoted-string = unterminated quoted string
cli-error-dialogue-catalog-reason-unsupported-escape = unsupported escape {$escape}
cli-error-decode-asset = failed to decode compiled asset {$path}: {$source}
cli-error-asset-metadata = failed to inspect compiled asset {$path}: {$source}
cli-error-asset-not-file = compiled asset path {$path} is not a regular file
cli-error-malformed-compiled-asset = malformed compiled asset: {$reason}
cli-error-diagnostics = diagnostics reported
cli-error-fixture-choice-index = fixture choice index {$index} is out of range for prompt {$prompt_keys} with {$choice_count} choices; indexes are 1-based
cli-error-fixture-choice-not-in-prompt = fixture choice `{$choice}` is not in prompt {$prompt_keys}
cli-error-ambiguous-fixture-choice = fixture block choice `{$block}` is ambiguous across {$prompt_count} prompts; use a line ID
cli-error-fixture-toml = failed to parse fixture {$path}: {$source}
cli-error-missing-path = input path does not exist: {$path}
cli-error-missing-fixture-choice = fixture is missing a [choices] entry for prompt {$prompt_keys}; supported keys for this prompt are listed in trace prompt.identity.fixture_keys
cli-error-no-inputs = no .recite inputs found
cli-error-output-overwrites-input = refusing to overwrite input {$input} with output {$output}
cli-error-blocking-effect = blocking effect `{$effect}` requires [effects].auto_ack_blocking = true in the fixture
cli-error-bench-json = failed to read or write benchmark JSON: {$error}
cli-error-trace-json = failed to encode trace JSON: {$error}
cli-error-schema-inspection-json = failed to encode schema inspection JSON: {$error}
cli-error-schema-inspection-unsupported-format = unsupported schema inspection format `{$format}` for {$path}
cli-error-schema-inspection-malformed = malformed {$format} schema input {$path}
cli-error-schema-inspection-invalid-summary = invalid schema inspection summary: {$reason}
cli-error-unknown-prompt = runtime emitted an unknown prompt line={$line} choices=[{$choices}]
cli-error-read = failed to read {$path}: {$source}
cli-error-read-dir = failed to read directory {$path}: {$source}
cli-error-write = failed to write {$path}: {$source}
cli-error-watch = {$message}

lsp-hover-requires = requires=(...) keeps the choice visible and marks it unavailable when the condition is false.
lsp-hover-if = :if structurally omits hidden dialogue content when the condition is false.
lsp-hover-speaker = Recite speaker `{$name}`.
lsp-hover-speaker-with-display-name = Recite speaker `{$name}` ({$display_name}).
lsp-hover-metadata = Recite metadata key `{$name}`.{$detail}
lsp-hover-metadata-with-domain = Recite metadata key `{$name}`. Values use metadata domain `{$domain}`.{$detail}
lsp-hover-condition = condition -> {$returns}
lsp-hover-effect = effect request -> {$modes}
lsp-hover-projection-query = projection query `{$name}` -> {$returns}
lsp-hover-presentation-projector = presentation projector `{$name}` with {$inputs} inputs, {$queries} queries, and {$outputs} outputs.
lsp-hover-presentation-output = presentation output `{$name}` from projector `{$projector}` -> {$target} {$kind}.
lsp-hover-presentation-label = presentation label `{$name}` with {$count} placeholder bindings.
lsp-hover-block = Recite block `{$name}` in the current project index.
lsp-hover-registry = Recite registry `{$name}`.{$detail}
lsp-hover-metadata-domain = Recite metadata domain `{$name}`.{$detail}
lsp-hover-availability-reason = Availability reason '{$name}'.{$detail}
lsp-hover-registry-value = Registry value '{$word}' in '{$name}'.{$detail}
lsp-hover-enum-value = Enum value '{$word}' in '{$name}'.
lsp-hover-domain-value = Metadata domain value '{$word}' in '{$name}'{$context}.{$detail}
lsp-hover-produced-by =  Produced by {$kind} `{$id}`{$label}.
lsp-hover-schema-producer =  Schema producer {$kind} '{$id}'.
lsp-hover-schema-freshness =  Content fingerprint { $fingerprint_state ->
    [present] {$fingerprint}
   *[absent] none {$fingerprint}
}; {$inputs} producer input fingerprints{$scope}.
lsp-hover-schema-freshness-state =  Freshness { $state ->
    [fresh] fresh
    [stale] stale
   *[other] unavailable
}: content { $content ->
    [fresh] fresh
    [stale] stale
   *[other] unavailable
}; manifest { $manifest ->
    [fresh] fresh
    [stale] stale
   *[other] unavailable
}; registries {$registries}; metadata domains {$metadata_domains}.
lsp-hover-schema-freshness-unavailable =  Freshness unavailable: { $reason ->
    [no-comparison-snapshot] no comparison snapshot
    [no-producer-metadata] no producer metadata
   *[other] unavailable for this client
}.
lsp-hover-schema-freshness-status = { $status ->
    [fresh] fresh
    [stale] stale
    [absent] none
   *[other] unavailable
}
lsp-hover-schema-scoped-fingerprints =  (scoped: {$fingerprints})
lsp-completion-availability-reason = parameterless availability reason
lsp-completion-block = Recite block
lsp-completion-speaker = Recite speaker
lsp-completion-metadata-key = Recite metadata key
lsp-completion-metadata-key-with-domain = Recite metadata key -> {$domain}
lsp-completion-metadata-domain = metadata domain `{$domain}`
lsp-completion-condition = condition -> {$returns}
lsp-completion-condition-documentation = Recite condition function
lsp-completion-effect = effect request -> {$modes}
lsp-completion-effect-documentation = Recite effect request
lsp-completion-projection-query = projection query call -> {$function}
lsp-completion-projection-query-documentation = Schema-owned presentation projection query function
lsp-completion-projection-query-function = projection query -> {$returns}
lsp-completion-projection-query-call = projection query call -> {$function}
lsp-completion-projection-input = projection input -> {$type}
lsp-completion-projector = presentation projector
lsp-completion-output = presentation output -> {$kind}
lsp-completion-label = presentation label template
lsp-code-action-insert-missing-id = Insert missing stable ID
lsp-code-action-insert-all-missing-ids = Insert all missing stable IDs in file
lsp-code-action-create-block-stub = Create block stub `{$block}`
lsp-code-action-add-condition = Add condition `{$name}` to schema
lsp-code-action-add-effect = Add effect `{$name}` to schema
lsp-code-action-schema-action = Schema capability ({ $declaration_kind ->
    [type] type
    [registry] registry
    [speaker] speaker
    [condition] condition
    [reason] reason
    [effect] effect
    [metadata-domain] metadata domain
    [metadata] metadata
    [projection-query] projection query
    [projector] projector
    [markup] markup
   *[schema] schema
} {$declaration_name}): { $action ->
    [open-source] open source declaration
    [edit-standalone] edit standalone source
    [invoke] invoke producer
    [retry] retry producer failure
    [read-only] read-only generated schema
    [unavailable] unavailable schema action
   *[other] schema action
} ({ $producer_state ->
    [present] {$producer}
   *[absent] no producer {$producer}
})
lsp-code-action-schema-disabled = Schema capability unavailable: { $reason ->
    [source-location] source location is not available
    [standalone-source-closed] standalone source is not open with a version
    [standalone-edit] standalone source edit is not available
    [producer-contract] producer execution contract is not available
    [generated-read-only] generated schema is read-only
    [unknown-source-owner] source owner is unknown
    [producer-capability] producer capability is unavailable
   *[other] schema action is not supported by this client
}.
lsp-warning-ui-config = UI configuration could not be loaded (code {$code}): {$detail}; using embedded en-US UI text.
lsp-client-start-failed = Recite language server could not be started: {$detail}.
lsp-client-error = Recite language server error: {$detail}.
lsp-client-exited = Recite language server exited: {$detail}.
lsp-client-restart-scheduled = Recite language server restart scheduled: {$detail}.
lsp-client-restart-exhausted = Recite language server restart attempts exhausted.
lsp-client-display-name = Recite
lsp-client-description = Language support for the Recite dialogue language.
lsp-client-untrusted-workspaces-description = Recite waits for workspace trust before starting a local language-server process.
lsp-client-configuration-title = Recite
lsp-client-configuration-path-description = Executable used to start the local recite-lsp language server. A bare name is resolved through PATH; a path is resolved relative to the project root.
lsp-client-configuration-args-description = Arguments passed to recite-lsp without a shell. The server currently speaks LSP over stdio.
lsp-client-configuration-project-root-description = Optional project root for language-server discovery. Relative paths are resolved from the first workspace folder. Leave empty to use the workspace folder.
lsp-client-action-stale = Recite code action is no longer applicable because the document changed.
lsp-client-action-closed = Recite code action is no longer applicable because the document closed.
lsp-client-action-reopened = Recite code action is no longer applicable because the document was closed and reopened.
lsp-client-action-expired = Recite code action expired before it was applied.
lsp-client-action-evicted = Recite code action was replaced by a newer action.
lsp-client-action-unknown = Recite code action is no longer available.
lsp-client-action-apply-failed = VS Code could not apply the Recite code action.
lsp-client-config-path-invalid = recite.lsp.path must be a non-empty string.
lsp-client-config-args-invalid = recite.lsp.args must be an array of strings.
lsp-client-config-project-root-invalid = recite.lsp.projectRoot must be a string.
lsp-client-config-project-root-needs-workspace = recite.lsp.projectRoot needs a workspace for relative paths.
lsp-client-not-running = Recite language server is not running.
