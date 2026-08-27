# Pull request

Recite uses GitHub for pull requests and project review:
https://github.com/plethu/recite

External code contributions are currently closed while the v1 architecture
settles. This template supports maintainer work and changes that were explicitly
invited; see `CONTRIBUTING.md` before opening a pull request.

This template is for a standalone maintainer pull request or the coordinator's
single stable-checkpoint milestone integration pull request. Delegated issue
slices are reviewed in isolated worktrees and mechanically integrated; they do
not open issue-slice pull requests.

Use a purpose-first `<kind>/<short-kebab-topic>` branch (never an
`issue-<number>-...` branch), and include the checks you ran below. The title
must match `[REC-N] <type>(optional-scope): <concise subject>`.
The body must include a `Closes`, `Fixes`, or `Resolves #N` token matching the
title's issue code.
For a standalone pull request, that code must match the one issue addressed by
every commit. For an integration pull request, apply the `workflow/integration`
label, use the coordinator's `integration/<short-kebab-topic>` branch, put the
milestone tracking issue's code in the title, and link the included issues
below; the commit range may contain multiple valid issue codes. Every commit
still needs its own `[REC-N]` conventional subject, at most one explanatory
body sentence, and no `Co-Authored-By:` or other agent-attribution trailer.

## Summary

<!-- What changed, and why? -->

## Delivery mode

<!-- Standalone PR, or milestone integration PR. If integration, name the
     milestone tracking issue and list the accepted issue slices. -->

## Checks

<!-- e.g. mise run verify -->
