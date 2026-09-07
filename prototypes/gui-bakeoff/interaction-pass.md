# Freya / GPUI writing trial

Freya was selected on 2026-09-08. This checklist is retained as historical
comparison material and a source of pending Freya platform checks; completing
the GPUI trial is no longer a prerequisite for starting the workbench. Both
entries are disposable, in-memory prototypes. Closing one discards its changes.
Use the supplied Alice scene; there is no project picker or Save yet.

## Start on Linux

Build from the repository root:

```sh
cargo build --locked --manifest-path prototypes/gui-bakeoff/Cargo.toml -p recite-bakeoff-freya
cargo build --locked --manifest-path prototypes/gui-bakeoff/candidates/gpui/Cargo.toml
```

Run one at a time:

```sh
python3 prototypes/gui-bakeoff/scripts/try-writer.py freya
python3 prototypes/gui-bakeoff/scripts/try-writer.py gpui
```

The Hyprland 0.56+ launcher prints a command to visit the app's workspace when
you are ready. It opens a floating 1200 × 800 window on a unique silent
workspace and waits until you close it. Ctrl+C also closes its owned process
group. It never switches your workspace or sends keyboard/mouse events.
Its temporary rule is disabled on exit; Hyprland retains disabled entries
until the next normal configuration reload.

## About ten minutes in each app

1. Rewrite Alice's first question. Add a hard newline and a long sentence that
   wraps. Change her speaker to Cheshire Cat after applying the draft.
2. Add a choice, name it “Let's try the garden”, and set its destination to
   garden. Try the scene and take that choice. Restart the preview and follow
   the original anywhere branch too.
3. Revise a line while its preview is open. Check that the preview becomes
   visibly stale. Apply and restart it; confirm that the revised line appears.
4. Undo and redo an applied edit using the toolbar. Separately, type in a
   passage and use Ctrl+Z. Move to another passage and use Ctrl+Z again:
   text from the previous passage must not appear there.
5. Leave an unapplied draft and try Source or another passage. You should
   get an understandable refusal and retain your text. Discard, then switch
   to Source. Change a prose line there, apply, and return to Script.
6. Repeat navigation with the mouse put aside. Use Tab, Shift+Tab, Enter and
   Space. Check that every control is reachable, focus is visible, the prose
   field can be escaped, and selecting a passage brings its editor into view.
7. Paste multiline text from another application, copy it back, and compare
   it. With your installed IME, compose Japanese, cancel once, then commit.
   Try “Café 🐈” and mixed Arabic/English text; inspect selection, deletion,
   caret movement and wrapping. Repeat the essential steps in Source.

Switch light/dark partway through. Record any lost text, unexpected history,
focus trap, misleading message, or extra step that interrupted writing.
The Freya RC's known Source preedit display gap remains recorded separately;
we are assuming its fix for final 0.5, not treating the pinned RC as fixed.

## Screen reader and macOS

These cells are **not run**. Toolkit tests do not substitute for either.

With the screen reader you normally use, identify the selected passage,
read/edit its prose, reach the speaker/destination controls, and hear draft
refusals and preview choices. Confirm field names, values, focus order and
state changes make sense without the screen. Record reader, OS, version and
the first concrete failing step.

On macOS, use a checkout containing these local prototype files and the
locked manifests. Try the same two build commands above; report any build
failure before counting an interaction result. Launch the resulting binaries
directly (the Hyprland launcher is Linux-only). Repeat steps 1–7 with the
platform's usual Command shortcuts, pasteboard, installed IME and VoiceOver.
No macOS build or runtime result has been established by this Linux pass.

## Record the result

Copy this small record for each candidate/platform; leave unrun cells unrun.

- Candidate and pinned version:
- OS / compositor / display scale:
- Keyboard layout / IME / screen reader:
- Writing task: not run / pass / fail — first failing step:
- Keyboard and undo: not run / pass / fail — what happened:
- System clipboard and IME: not run / pass / fail — what happened:
- Screen reader: not run / pass / fail — what happened:
- Which felt easier to write in, and why:
- Anything that would prevent everyday use:

Choose after reviewing the failures and your writing preference. A crash,
lost text or focus trap needs a fix and a repeat of that step. An accessibility
gap needs a concrete resolution before accepting that platform. Cosmetic
differences alone should not restart framework research.
