# Writer acceptance

Writer is a preview. #54 records the framework decision and #123 defines the
conformance contract. Keep #170 open until native authoring and accessibility
requirements have evidence; installed-package acceptance belongs to #79.
Automated tests and captured renders do not establish physical input,
screen-reader usability, GPU performance, or cross-platform support.

## Repeatable checks

Run from the repository root:

```sh
mise exec -- just check-writer
mise exec -- just check-writer-accessibility
mise exec -- just probe-writer-native-accessibility
```

The focused accessibility checks also run in the full Writer gate. They cover
control names, modal metadata, forward/reverse keyboard focus, focus restoration,
100% and 200% UI scale at 900 × 650, keyboard-only source editing, rebinding,
Vim modes, pickers, scrolling, synthetic text input, contrast and reduced motion.
Status metadata is polite for progress and assertive for errors; tests do not
prove that a native screen reader announces it.

| Requirement | Automated evidence | Native acceptance |
| --- | --- | --- |
| Keyboard, focus, escape and retry | `accessibility.rs`, `commands.rs`, `keybindings.rs`, `picker.rs`, `writing_workspace.rs` | Repeat the keyboard and recovery drills below |
| Names, roles, states and announcements | Accessibility tests and the AT-SPI probe | Screen-reader navigation, edit, error and progress announcements |
| Source round-trip, IDs, diagnostics, preview and localisation | Writer/model tests, `project_workflows.rs`, `localisation.rs`, `preview.rs`, `preview_locale.rs` | Complete a real writing/localisation session |
| IME and mixed-direction text | `text_input.rs` synthetic input probes | Composition, caret, selection and copy/paste with real input methods |
| Contrast, non-colour cues, scaling and motion | Palette contrast and component/layout tests | High-contrast settings, display scaling and reduced-motion settings |
| Graph/text equivalence | Map, script and navigation tests | Perform essential authoring actions without pointer input |
| Packages and desktop activation | Package checkers and platform build jobs | [Install, upgrade, uninstall and link matrix](packaging.md) |

Test paths above are under `crates/freya/tests` unless otherwise noted. Palette
and component tests live beside their private implementation modules.

## Known limits

- On September 23, the Linux AT-SPI probe found the named button but could not
  focus or invoke it. Freya 0.5.0-rc.7 ignores native `ActionRequested` events.
  The probe reports `limited`; `--require-actions` fails until focus and
  invocation affect the application. This is a known blocker, not a skipped test.
- The pinned CodeEditor exposes an unnamed inner TextInput and does not display
  IME preedit. `probe_records_missing_code_editor_preedit_presentation` records
  the gap; a passing test does not mean IME support passes. After upgrading
  Freya, replace it with a positive assertion once fixed. Multiline Input has
  a separate positive composition test. The existing upstream report is
  [Freya #2248](https://github.com/marc2332/freya/issues/2248); the maintainer asked
  us to pause duplicate upstream patch work.
- Exact prose caret/selection restoration after Input remounts is deferred
  pending a supported upstream approach; see
  [Freya #2308](https://github.com/marc2332/freya/issues/2308). Draft retention
  and Source caret restoration remain required.
- The backend does not forward native trackpad pinch. Ctrl+scroll zoom tests
  do not establish physical gesture support or smooth trackpad scrolling.
- ARM Linux, native macOS/Windows, screen readers, physical IME/BiDi, display
  scaling and sustained large-project sessions still need recorded host evidence.

The AT-SPI probe uses a private D-Bus session and temporary preferences. It needs
Linux AT-SPI/Python bindings and a display or Xvfb. Neither it nor package
inspection installs the app or changes the user's desktop associations.

## Start here: a real writing session

Use a copy of a real project. Launch from the repository with
`mise exec -- just writer`, then open that project.

Spend 30–45 minutes doing ordinary work:

1. Create a beat, write several lines and replies, change destinations and use
   previews. Navigate with the sidebar, graph and history.
2. Undo and redo, switch Map/Source, edit source, apply it, save and reopen.
3. Open a PO catalogue, translate several passages, mark one reviewed, use the
   queue, and leave drafts in more than one beat.
4. Quit. The dialog should name those drafts and offer working navigation,
   save-all and discard-all actions. Cancel once, then save and quit.
5. Reopen and verify the saved text and destinations.

Report anything that made you hunt for a control, lose your place, or wonder
whether work was saved. Include the steps and the expected behavior.

## Recovery and conflict drill

Use disposable copies and one writer window per file.

1. Change a translation without saving. Wait two seconds, terminate that writer
   process forcibly, then reopen the same catalogue. The draft should return.
2. Bind a standalone declaration TOML source. Leave incomplete TOML, wait, force
   termination, and reopen and bind the same source. The incomplete text should
   return. Source association itself is not persisted.
3. Repeat with a manuscript field draft.
4. With a recovered draft open, change the corresponding file externally.
   Saving must report the conflict and retain the draft, not overwrite the other
   editor's work.
5. Discard a recovered draft, reopen again, and confirm it stays discarded.

Recovery is asynchronous: a process killed before the next snapshot reaches disk
can lose the latest edit. It must not corrupt the last durable snapshot.

## Keyboard, size and motion

At a normal window size and about 900 by 650, repeat the important steps using
only the keyboard. Test Tab/Shift+Tab, Enter, Escape, the displayed shortcuts,
search, destination selection, menus and every close-dialog action.

Expected: visible focus, no focus behind a modal, a way out of every menu, and
focus returning somewhere useful. Repeat with the Vim keymap; typing in an editor
must not trigger navigation shortcuts.

Try both appearances, reduced motion, 200% display scaling and long paragraphs
with long scene names. Controls must remain reachable and text must not cover
actions. Reduced motion should remove movement without hiding state changes.

## Native accessibility and input

On each platform claimed by the milestone (Linux, Windows and macOS), record OS,
display scale, and the input/accessibility tools used:

- Screen reader: navigate the sidebar, edit text, select destinations, open a
  dialog and provoke an error. Names, selected states, errors and focus changes
  should be announced. Check that graph navigation has a usable textual route.
- IME: compose and commit text in a manuscript field, source and translation.
  Candidate windows should follow the caret; composition should not submit a
  dialog or invoke shortcuts.
- Mixed RTL/LTR: edit a real Arabic or Hebrew sample with punctuation and Latin
  identifiers. Check caret movement, selection, copy/paste and reopened text.
- High contrast and display scaling: verify selection, focus, errors and disabled
  states remain distinguishable.

Unavailable hardware or an unfamiliar input method is an untested item, not a
pass. Send screenshots or a short recording for visual/focus problems, plus exact
reproduction steps. Do not send private project content unless intended.

## Acceptance record

For each session, record date, OS, build/commit or dirty-checkout identity,
window size/scale, result, and reproduction steps for failures. Automated gate
results are reported separately in the implementation handoff. No packaging,
installer, signing or distribution acceptance is implied.
