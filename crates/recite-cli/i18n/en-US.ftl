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
tui-choice-input-prefix = choice id/index>
tui-choice-input = choice id/index> {$input}
tui-condition-yes-row = yes
tui-condition-no-row = no
tui-condition-yes-shortcut-row = (y)es
tui-condition-no-shortcut-row = (n)o
tui-ack-enter-hint = Press Enter to acknowledge

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
tui-transcript-deferred = deferred
tui-transcript-end = end
tui-transcript-selected = selected
tui-transcript-completed = completed
tui-transcript-deferred-effects = deferred effects

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
tui-help-description-submit-effect = acknowledge the blocking effect
tui-help-description-finished = leave the finished play screen
tui-help-description-command = enter command mode
tui-footer-command = Enter runs command | Esc cancels

cli-error-play-eof = reached EOF while reading {$field}
cli-error-play-invalid-input = invalid play input: {$message}
cli-error-play-interrupted = play interrupted
cli-error-play-tui-requires-terminal = recite play --ui tui requires interactive stdin and stdout; use --ui plain for pipes, CI, or accessibility tools
cli-error-ui-config-read = failed to read UI config {$path}: {$source}
cli-error-ui-config-toml = failed to parse UI config {$path}: {$source}
cli-error-ui-locale-invalid = failed to parse UI config {$path}: invalid [ui].locale `{$locale}`; expected a BCP-47 locale such as "en-US" or "system"
