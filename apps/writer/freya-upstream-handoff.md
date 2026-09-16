# Freya IME follow-up

Updated 2026-09-07. [Freya #2248: CodeEditor does not display IME preedit
text](https://github.com/marc2332/freya/issues/2248) is being handled by Marc.
The Recite maintainer confirmed this and asked us to pause our upstream fix
work. No fix branch or PR was created. Do not start a competing patch, post
status comments, or chase the issue automatically. This note supports answering
questions the maintainer brings back to this session or a later one.

## What was actually demonstrated

- crates.io Freya and freya-testing **0.5.0-rc.4**, Rust
  `1.96.0 (ac68faa20 2026-05-25)`, CachyOS Linux rolling, x86_64.
- Injecting `PlatformEvent::ImePreedit` with `にほん` into a focused CodeEditor
  produces no rendered paragraph containing that composition text.
- The equivalent event in multiline Input produces a visible preedit span
  while leaving the bound text unchanged. Clearing preedit and supplying
  committed text updates the value. Newlines and committed Unicode work.
- The exact test pasted into the issue was compiled and run separately on
  2026-09-07. It failed at its composition-visibility assertion, as reported.
  The temporary failing test was then removed from the normal test suite.
- Upstream main at `bf825f733fdebc26963e4a1f62621ff4816a1286` was inspected:
  CodeEditor wires keyboard handlers but no preedit handler; Input handles
  preedit and renders composition segments. **That main revision was not built.**

These are headless event/rendering observations. They do not establish behaviour
with a physical Japanese IME, candidate-window placement, platform-specific
composition sequences, screen readers, or every commit/cancel/undo path.
Do not describe them as installed-IME acceptance.

## Reproduction and local evidence

The [audited issue body](upstream-code-editor-preedit.md) contains the exact
published reproduction, dependencies, expected behaviour, and environment.
It was checked against the live published body after editing.

[Text-input probes](crates/freya/tests/text_input.rs) contain:

- `probe_records_missing_code_editor_preedit_presentation`: asserts the
  **known absence** of preedit presentation. A pass records the gap; it does
  not mean IME support passes. After an upstream fix, reassess this probe and
  replace it with a positive regression assertion.
- `multiline_input_presents_preedit_without_committing_it`: positive control
  for composition presentation and unchanged draft text, followed by committed
  Japanese text, a newline, and `Café 💬`.
- `committed_unicode_and_hard_newlines_reach_the_buffer`: CodeEditor committed
  Unicode, newline and backspace behaviour.

From the Recite root:

```sh
mise exec -- cargo test --locked --manifest-path apps/writer/Cargo.toml -p recite-writer --test text_input
```

Autofocus must settle with `test.sync_and_update()` before injecting the first
event. The application interaction tests also allow layout updates after view
changes. An early test failure caused by unsettled autofocus was corrected
locally and was not reported upstream.

## Correction to the first bake-off assessment

Our first pass missed `Input::multiline(true)` in the same pinned Freya release.
Input already supports wrapping and preedit presentation. The Script adapter
now uses multiline Input; Source retains CodeEditor for syntax highlighting.
Do not repeat the earlier broad impression that Freya itself lacks multiline
wrapping or composition handling. CodeEditor is the scope of #2248.

The README and evidence ledger now reflect this correction; use
this note and the current adapter/tests for IME follow-up. The whole-scene
prototype work remains local and unfinished; it is separate from Marc's fix.

## Contribution rules checked

- [CONTRIBUTING.md](https://github.com/marc2332/freya/blob/main/CONTRIBUTING.md)
  describes setup and upstream checks (`just c`, `just f`, `just t`).
- The [bug-report form](https://github.com/marc2332/freya/blob/main/.github/ISSUE_TEMPLATE/bug_report.yml)
  requires description, reproduction, expected behaviour, Freya/Rust versions
  and OS. The issue was revised to match those fields. The form's `bug` label
  could not be applied because GitHub reported that label did not exist.
- The [PR template](https://github.com/marc2332/freya/blob/main/.github/pull_request_template.md)
  explicitly includes an AI-assistance disclosure checkbox. No PR was submitted.

The first issue submission preceded the contribution-policy audit. Its body
was subsequently edited using `avoid-ai-writing`, aligned with the issue form,
and verified against GitHub. Future public replies must pass those checks
**before** submission. Re-read current upstream guidance if a later reply or
contribution is requested; this note is a dated record, not permanent policy.

## Handling a follow-up question

Start from Marc's exact question and the issue's current state. Separate what
the tests prove from what needs a new test. Reproduce any new claim locally;
do not invent an IME/OS result to fill a gap. Prepare a short, specific answer
for the maintainer to review, and disclose assistance as upstream requires.
The request to retain this note does not authorize sending further messages.
