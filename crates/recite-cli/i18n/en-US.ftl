play-tui-starting = starting recite play TUI; use --ui plain for line-oriented output

play-start = play asset={$asset} block={$block}
play-line = line {$id}: {$text}
play-prompt-line = prompt {$id}: {$text}
play-prompt = prompt
play-choice-row =   [{$index}] {$id}: {$text}{$availability}
play-choice-unavailable-suffix =  (unavailable)
play-choice-prompt = choice> 
play-condition-prompt = condition {$query} [y/n]> 
play-condition-result = condition {$query} = {$result}
play-selected-choice = selected choice {$id}
play-effect = effect {$mode} id={$id} function={$function} args={$args}
play-ack-prompt = ack {$id} with Enter or `ack`> 
play-ack-completed = acknowledged effect {$id} completed
play-end = end
play-deferred-effects = deferred effects:
play-deferred-effect-row =   {$function} {$args}
play-invalid-input = invalid input: {$message}

play-error-enter-y-or-n = enter y or n
play-error-press-enter-or-ack = press Enter or type ack
play-error-empty-choice = choice selection cannot be empty
play-error-choice-index-out-of-range = choice index {$index} is out of range; enter 1-{$count} or a choice ID
play-error-choice-id-invalid = invalid choice ID `{$id}`: {$error}
play-error-choice-id-unavailable = choice ID `{$id}` is not available here
play-error-choice-unavailable = choice `{$id}` is unavailable
play-error-choice-unavailable-reason = choice `{$id}` is unavailable: {$reason}

tui-ready = ready
tui-finished = finished
tui-command = command:
tui-command-with-value = command: :{$command}
tui-unknown-command = unknown command: :{$command}
tui-normal-mode = normal mode
tui-choice-status-standard = choice> arrows move, type ID/index, Enter selects
tui-choice-status-vim = choice normal> j/k or arrows move, i types ID/index
tui-choice-input-prefix = choice id/index> 
tui-choice-input = choice id/index> {$input}
tui-condition-input-prefix = condition> 
tui-ack-status = ack {$id} with Enter or ack
tui-ack-input-prefix = ack> 

tui-header-title = recite play
tui-header-asset = asset
tui-header-block = block
tui-waiting = Waiting for the next runtime event...
tui-condition-title = Condition
tui-effect-title = Blocking Effect
tui-choice-title = Choose a branch
tui-metadata-mode = mode
tui-metadata-runtime-effect-id = runtime effect ID
tui-metadata-function = function
tui-metadata-args = args
tui-input-answer = answer
tui-input-ack = ack
tui-input-choice = choice
tui-choice-unavailable =  unavailable
tui-choice-unavailable-reason =  unavailable: {$reason}

tui-transcript-line = line
tui-transcript-prompt = prompt
tui-transcript-choice = choice
tui-transcript-condition = condition
tui-transcript-effect = effect
tui-transcript-ack = ack
tui-transcript-end = end
tui-transcript-selected = selected
tui-transcript-completed = completed
tui-transcript-deferred-effects = deferred effects

tui-help-label = help 
tui-help-choice = arrows move, Enter selects, type an ID/index, :q quits
tui-help-condition = enter y/n, :q quits, Ctrl-C quits
tui-help-effect = Enter or ack completes, :q quits, Ctrl-C quits
tui-help-default = :q quits, Ctrl-C quits
tui-footer-compact-choice = Enter selects | arrows move | ? help
tui-footer-compact-condition = y/n | ? help
tui-footer-compact-effect = Enter/ack | ? help
tui-footer-compact-finished = Enter/Esc/q exit
tui-footer-choice-normal = Enter selects highlighted choice | i types ID/index | :q quits | ? help
tui-footer-choice-insert = Type choice ID/index or use arrows | Enter submits | :q quits | ? help
tui-footer-command = Enter runs command | Esc cancels
tui-footer-help = Esc closes help
tui-footer-condition = Enter y or n | :q quits | ? help
tui-footer-effect = Enter or ack acknowledges | :q quits | ? help
tui-footer-finished = Enter/Esc/q to exit

cli-error-play-eof = reached EOF while reading {$field}
cli-error-play-invalid-input = invalid play input: {$message}
cli-error-play-interrupted = play interrupted
cli-error-play-tui-requires-terminal = recite play --ui tui requires interactive stdin and stdout; use --ui plain for pipes, CI, or accessibility tools
cli-error-ui-config-read = failed to read UI config {$path}: {$source}
cli-error-ui-config-toml = failed to parse UI config {$path}: {$source}
cli-error-ui-locale-invalid = failed to parse UI config {$path}: invalid [ui].locale `{$locale}`; expected a BCP-47 locale such as "en-US" or "system"
