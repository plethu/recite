# GUI visual language

Accepted visual baseline for the native GUI bake-off, September 2026. The
maintainer approved the revised writer-facing Script and highlighted Source direction. It does not select a toolkit
or claim that a workbench exists.

Open [the visual reference](design/visual-language.html) in a browser. It has
light, dark, and system appearance, text scaling, and alternate status examples.
Its Script/Source switch and expandable details demonstrate progressive disclosure;
the writing surface itself is a specimen, not a functioning authoring tool.

## Direction

Recite should feel like a place to work on a script: quiet, literate, and warm,
with enough structure to keep a complicated scene understandable. The writer's
words occupy the most prominent surface. Scenario and quest writers should
be able to create dialogue, connect choices, and test a scene without learning
Recite punctuation or managing localisation IDs. That is the workbench's
reason to exist alongside capable editors that already have the Recite LSP.

The README's plain-text scenes and literary opening provide the starting point.
Use a clean reading surface, modest headings, generous dialogue leading, and
small editorial details. Avoid simulated paper, distressed type, theatrical
ornament, chat bubbles, and decorative portraits. Recite serves many games;
their art direction belongs to them.

Use warm ivory and stone in light mode, neutral charcoal in dark mode. Text
and separators are neutral too. Sage marks selected places and primary
actions; it must not tint the whole application. Peach distinguishes the
player-choice area, amber marks attention, and source syntax has its own
blue, violet, teal, and ochre roles. Neither mode is a colour inversion.

The interface serves sustained work, including tired or interrupted work.
Keep controls predictable, status persistent, and recovery choices visible.
No engagement counters, streaks, unsolicited assistant surfaces, or promotional
empty states. A writer should be able to leave with their ordinary project
files and continue in another editor.

## Writer-facing authoring

The default is a structured script, with dialogue flowing down the page. A
writer chooses a speaker and writes in a normal text field; adds a choice and
picks its destination; selects a condition or effect from the project's schema
and fills in typed values. In the eventual workbench, these are real editing
controls, with undo, keyboard access, multiline entry, and IME support. The
reference illustrates their placement without pretending to implement them.

| Writer's task | Default presentation | Details available on demand |
| --- | --- | --- |
| Write dialogue | Speaker picker and multiline dialogue field | Source location, line label, stable localisation ID |
| Offer a choice | Choice text and destination picker | Choice ID, underlying target, availability condition |
| Add a condition | Schema-backed predicate picker and typed arguments | Exact predicate, expression, source, and schema help |
| Request an effect | Schema-backed operation and argument fields, with timing | Exact call, types, provenance, and runtime trace |
| Connect a scene | Named sections, destination links, incoming/outgoing list | Block identifier and raw divert statement |
| Repair a problem | Explanation beside the affected line or choice | Diagnostic code, full source range, related locations |
| Try a path | Play through from the selected section; set fixture values | Condition answers and emitted effect requests |

Hide stable IDs from routine writing, not from the user. An expandable details
area exposes them with an explanation and a copyable value. New content gets
IDs through the shared source-edit operation; existing IDs never change just
because someone edits text, reorders a line, or switches views. Copy/duplicate
must request a fresh identity where required, never clone a frozen ID blindly.
This is an implementation requirement, not a claim that every operation exists.

Friendly display names may come from existing project data or presentation of
an identifier. The sample renders `which_way` as “Which way” and `anywhere` as
“Anywhere”; it adds no stored title syntax. Where there is no authored display
name or help, show the actual schema name and typed arguments. Do not invent
natural-language explanations of game logic or imply that choosing an effect
executes it in the game.

Script and Source are two views of the same document. A view switch retains
the current line, selection, unsaved work, and undo history. Structured edits
must be local, lossless source transactions through the shared authoring
kernel, preserving comments, formatting, unknown content, and IDs outside the
edit. Do not regenerate an entire file from form state. Incomplete or
unsupported syntax stays visibly present as an exact source fragment with
an explanation and a source-edit route; never silently omit it or overwrite it
with a simplified form. Valid neighbouring content should remain usable.

The initial common authoring slice should prove speaker/text editing, adding
a choice with a destination, and a schema-backed condition/effect example.
Source inspection and round-trip preservation are part of its evidence.
Production breadth remains milestone work; this design neither introduces a
second parser nor makes a general graph editor a prerequisite.

## Shared palette

These are initial reference values, expressed as sRGB hex colours. Roles are
the common contract across toolkits. The matching CSS variables in the visual
reference are a translation of this table, not a production theme API.

| Role | Light | Dark | Use |
| --- | --- | --- | --- |
| `canvas` | `#F5F2ED` | `#202124` | Window ground and quiet margins |
| `surface` | `#FFFDFA` | `#292A2D` | Source and reading surfaces |
| `subtle` | `#ECE8E2` | `#303237` | Grouped controls and secondary regions |
| `text` | `#322F2B` | `#EAE7E1` | Body, source, labels |
| `text-muted` | `#65615B` | `#BCB8B1` | Paths, metadata, secondary explanation |
| `border` | `#827D75` | `#A29F99` | Operable control boundaries |
| `accent` | `#48634D` | `#B4CBA4` | Sage: current location, links, primary action |
| `on-accent` | `#FFFDFA` | `#202124` | Text on solid primary action |
| `selection` | `#E0E8DC` | `#3E4D40` | Selected row or range; use `text` over it |
| `peach` | `#F1D4BF` | `#E5BDA1` | Choice/reading detail; dark ink over it |
| `amber` | `#F1DC9C` | `#E6C579` | Attention background; dark ink over it |
| `warning` | `#755017` | `#E6C579` | Attention text on neutral surfaces |
| `error` | `#963F36` | `#F2B3A6` | Errors, accompanied by a label and location |
| `focus` | `#48634D` | `#B4CBA4` | Keyboard focus outside the control |
| `syntax-control` | `#365F92` | `#A6C8F0` | Statement markers, directives, reserved words |
| `syntax-label` | `#72528B` | `#D2B5EC` | Block/line/choice labels and destinations |
| `syntax-call` | `#266967` | `#8ACFC6` | Condition/effect calls and runtime bindings |
| `syntax-value` | `#865520` | `#E6BE85` | Metadata values, literals, strings |

Peach is a small accent around choices, not the background of every panel. Keep most
of the window neutral. Amber never means success. Sage selection never means
validation passed: write “No diagnostics” when that is what the system knows.
Colour must not be the only distinction between selected, focused, stale,
unavailable, and invalid.

Target at least 4.5:1 contrast for all ordinary text, including muted text, and
3:1 for control boundaries and focus against their neighbours. Measure actual
pairings in each candidate; opacity and native widgets can change the result.
Peach and amber fills use the light theme's dark `text`, in both modes.
Separators that carry no essential information may be softer than controls.

Respect the platform's high-contrast setting. Replace authored colours with
system colours where necessary, retain outlines and textual markers, and do
not insist on sage or peach when those colours weaken access. Light and dark
are required for the initial bake-off. A user theme editor, import format, and
theme marketplace are out of scope; semantic roles leave room for custom
themes later without settling their configuration or compatibility contract.

## Type and rhythm

| Surface | Reference | Treatment |
| --- | --- | --- |
| Interface | Platform UI sans, 14–16 logical px | Regular body; medium labels; 1.4 line height |
| Source and trace data | Platform monospace, 15 logical px | 1.6 line height; ligatures off by default |
| Script dialogue and preview | Readable serif, 20 logical px | 1.55 line height; about 50–65 characters per line |
| Pane heading | UI sans, 16 logical px | Medium or semibold; sentence case |
| Project/scene heading | Serif, 24–28 logical px | Used once per relevant reading context |
| Supporting text | UI sans, 14 logical px | Full readable contrast; no tiny metadata |

The specimen uses locally available system families, with Georgia/Noto
Serif/DejaVu Serif and system monospace fallbacks. No font download is needed.
The serif is a reading treatment; source remains monospaced. Let users choose
their source and reading fonts when the workbench grows. For the bake-off,
record actual fonts and fallback coverage, particularly for non-Latin text.
Do not force uppercase speaker names or add tracking to translated labels.

Use a 4-unit spacing step: 4, 8, 12, 16, 24, 32. Default control heights are
32–36 logical px, with room to grow at larger text sizes; keep interactive
targets at least 24 by 24 logical px. Use roughly 12–16 units of pane padding
and 24 around dialogue. Radii are restrained: 4 for controls, up to 8 for a
floating surface. Pane boundaries are straight. Shadows belong to temporary
overlays and must not be the only way to see their boundary.

Use simple outline icons at 16–20 units, accompanied by labels for important
actions. Platform icon sets may vary. No icon font, decorative status glyphs,
or shape that requires remembering a legend to save or repair a scene.

## Composition

At a comfortable desktop width, put scene/section navigation on the left and
the structured script in the centre. Show contextual help or details for the
selected item on the right when useful; keep it collapsible. The script gets
the largest share. Preview is an explicit “Try this scene” workflow, not a
permanent duplicate transcript consuming a third of the writing space. Keep a
small persistent status area for saved state and preview freshness.

A section heading, speaker labels, dialogue, choices, and destinations should
be distinguishable before reading their content. Use neutral rules between
passages, a sage leading mark for the current passage, and a peach leading
mark and “Player choice” label for choice groups. Conditions and effects have
explicit textual labels and argument controls; neither becomes an unexplained
symbol. Insertion actions appear between passages and at the end, and remain
reachable without hover or dragging. Avoid a stack of boxed form cards.

Default navigation uses scene and section names. Paths, file extensions, line
numbers, diagnostic codes, and ID strings belong in Source or details. Keep
source available through a clearly labelled view switch, without making it a
prerequisite for the supported writing tasks. Native menus, window chrome,
file pickers, and shortcuts follow their platform.

Panes must be resizable and collapsible in candidate implementations, with
keyboard alternatives to dragging. At narrow widths or large text sizes, show
one working pane and explicit navigation to the others, retaining edits,
selection, and focus. The browser sheet stacks panes for inspection; that is
not a prescribed desktop navigation implementation.

Source highlighting follows production spec §14: blue structural markers and
reserved words, violet labels and destinations, teal calls/runtime bindings,
ochre values, neutral dialogue, and readable secondary ink for comments and
stable IDs. Metadata keys use secondary ink and remain distinct from values.
Preserve punctuation and indentation; the colours reinforce visible syntax.
Keep active-line background, text selection, search matches, and diagnostics
legible without replacing this hierarchy. Errors get an underline/marker and
a message with location, never just another syntax colour. The sample's
coloured spans are illustrative; actual clients use existing grammar/kernel
classifications rather than copying an HTML highlighter.

Preview reads like the script, with selectable choices and an explicit dialogue
locale. The UI locale is a different preference. No typewriter effect, automatic
scrolling away from the reader, or automatic choice selection. Fixture values
and unmet conditions explain why a path is available. Trace details describe
condition answers and effect requests; they never assert game-side execution.

Graph nodes share source/outline names. Use labelled edges and an equivalent
list of connections. Schema and PO views use the same readable tables,
source links, and labelled state; generated manifests remain visibly read-only.

## States and language

| Situation | Visible treatment and example | Behaviour |
| --- | --- | --- |
| Current item | Sage background, leading rule, selected state | Selection and focus remain independently visible |
| Keyboard focus | 2-unit outline with 2-unit gap | No clipping; visible on primary actions as well as rows |
| Unsaved source | “Unsaved changes” beside file identity | Persists until saved; never just a dot |
| Build pending | “Checking changes…” | Preserve writing focus; offer cancellation for long operations |
| Stale preview | Amber detail: “Preview is out of date” | Keep last result with its status; offer rebuild |
| Diagnostic | Explanation attached to the affected passage | Repair there; source location and code in details |
| Save conflict | “File changed on disk” and explanation | Preserve buffer; offer comparison before replacement |
| Failed operation | “Could not save” plus reason | Retain work; expose retry and an alternative destination |
| No open project | “Open a project” and short explanation | Useful action; no tips carousel or demo metrics |
| No diagnostics | Plain “No diagnostics” | Quiet confirmation, no celebratory animation |
| Unavailable action | Visible label and reason | Explain where it can be enabled; do not rely on faded text |

These are draft English copy samples. Candidate implementations use the shared
[UI localisation contract](ui-localisation-contract.md); the specimen is not
a second string catalogue or a claim that these message IDs already exist.
Prefer direct verbs: Open project, Save, Compare changes, Rebuild preview.
Avoid blame, jokes about errors, and internal compiler terminology where a
plain explanation would do. Technical detail remains available for inspection.

Use in-place feedback for routine results. Reserve modal interruption for a
decision that cannot safely wait. Important information remains visible until
resolved; no toast-only errors. Announce relevant asynchronous changes once,
without stealing focus or reading every keystroke. After a transient surface
closes, restore focus to its invoker or a sensible surviving control.

Routine input and navigation respond immediately. Optional hover or panel
transitions may take 100–150 ms; no decorative motion is required. Reduced
motion removes those transitions without removing status information.

## Applying this to the bake-off

Every entry uses the same project and authoring fixtures from #54/#123. The
Alice exchange in the specimen is taken from the repository README, which
identifies its public-domain source. It is a visual sample, not a substitute
for executable shared conformance fixtures.

Required common ground:

- Both light and dark appearance, identical content and information hierarchy.
- A structured writing surface as the default, with speaker, dialogue, choice,
  and destination controls. No routine task requires reading a frozen ID.
- A Source view with distinct syntax categories and a details route for IDs;
  switching views preserves source, identity, unsaved work, and undo history.
- The semantic colour roles, readable text sizes, script/source distinction,
  persistent status, and labelled recovery actions described here.
- Current, unsaved, pending, stale, diagnostic, and save-conflict examples;
  selected and keyboard-focused controls must be distinguishable.
- Narrow-window and 200% text-size evidence, long paths, expanded UI labels,
  and mixed-direction text. Treat these as tests, not supported-locale claims.
- Actual platform high-contrast, keyboard, screen-reader, and IME evidence
  under #123. A screenshot cannot establish those outcomes.

Allowed variation includes native menu and dialog appearance, system fonts,
scrollbars, shortcut notation, window decorations, and suitable native control
metrics. Record differences and their reason. Do not replace a well-behaved
native control merely to match the specimen's pixels. An inaccessible custom
control cannot earn credit by looking closer to this reference.

Have scenario/quest writers try adding a speaker line, making a branch,
changing a destination, correcting a missing destination, and trying a path.
Record assistance needed, notation/ID exposure, recovery mistakes, and any
forced switch to Source. An attractive source editor alone does not satisfy
this authoring test. Report observed usability separately from automated checks.

Capture matching states at the same logical viewport and scale, naming OS,
toolkit version, font, theme, and text size. Record missing behaviour separately
from visual differences. Compare legibility and authoring effort, including
the effort needed to reproduce the design. This brief adds no separate parser,
editor engine, production component library, or custom theming system.

Authority remains with [production spec §§14–15](recite-production-spec.md)
and the [roadmap's GUI gates](roadmap.md). The maintainer accepted this revised
direction before the first candidate implementations.

## Reference verification

Checked on Linux in headless Chromium 149.0.7827.55 on 2026-09-07. Both Script
and Source were checked in light and dark appearance at widths of 360, 900,
and 1440 CSS pixels and at 100% and 200% text size: 24 combinations, with no
document-level horizontal overflow. Source intentionally scrolls horizontally.
Script is the initial view; frozen IDs are absent from visible text until
line details open, and disappear again when those details close. The Source
view renders five distinct colour roles for structure, labels, calls, values,
and stable IDs. System-dark preference and emulated forced colours were checked.

Calculated sRGB contrast for text, including syntax colours on neutral surfaces,
is at least 5.03:1 in light mode and 6.49:1 in dark mode. Border/focus pairings
on neutral and selection backgrounds reach at least 3.25:1 and 3.39:1
respectively. These measurements cover the opaque reference values, not
future native widgets or arbitrary user themes.

Git policy, local-link, and whitespace checks passed. No Rust runtime code
changed, so the Rust workspace gate was not run. Actual writing, source/view
synchronisation, undo, native screen-reader announcements, and IME/BiDi
conformance are requirements for candidate implementations, not capabilities
proved by this design sheet. Writer usability sessions have not yet happened.
