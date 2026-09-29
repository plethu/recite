---
title: Ink imports
description: Inspect Ink source and convert the documented knot and divert subset.
---

```sh
recite import fixtures/import/knots.ink --from ink
```

The checked subset accepts named knots (`=== Start ===`), plain lines, static
`-> Target` diverts, `-> END`, and sticky menu-only choices written on one line:

```text
=== Start ===
The station is quiet.
+ [Listen] -> End
=== End ===
The signal returns.
-> END
```

A choice group must follow a dialogue line and terminate the knot. The square
brackets make the choice text menu-only; the generated Recite choice therefore
does not add an echoed dialogue line. Once-only `*` choices are held back because
Recite does not inherit Ink's visit history. Plain `//` comment lines are ignored.

Stitches, gathers, weave, glue, tunnels, threads, variables, expressions, tags,
external calls, includes and output-mixed choices are outside the subset. An
affected knot is held back as a whole and reported with its source spans.
Top-level flow and declarations are reported rather than treated as a default
knot. Missing targets in the generated result fail native validation.

Map state queries to native schema conditions and game work to typed effects
manually. Ink runtime JSON and save files are not import inputs. Review every
[report item](/migration/importer-boundaries/) before adopting generated source.

Syntax references: inkle's [Writing with ink](https://github.com/inkle/ink/blob/master/Documentation/WritingWithInk.md)
and [runtime integration guide](https://github.com/inkle/ink/blob/master/Documentation/RunningYourInk.md).
Support is limited to the checked source forms above, not all features of an Ink
release.
