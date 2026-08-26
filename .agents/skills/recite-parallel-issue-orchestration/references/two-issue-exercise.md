# Bounded Two-Issue Orchestration Exercise

Recorded 2026-08-26 against the GitHub history for Recite's release-hardening
work. This is evidence for the overlay, not a reusable workflow recipe.

## Inputs

The coordinator ran two independent issue streams:

| Issue | Purpose-first branch/PR | Protected merge |
| --- | --- | --- |
| [#142](https://github.com/plethu/recite/issues/142) | `ci/pin-ripgrep` / [#144](https://github.com/plethu/recite/pull/144), then `ci/pin-rust-components` / [#146](https://github.com/plethu/recite/pull/146) | signed squashes `e84176a`, `2d4c7a4` |
| [#143](https://github.com/plethu/recite/issues/143) | `ci/git-policy` / [#147](https://github.com/plethu/recite/pull/147) | signed squash `1e45889` |

The write scopes were separate: hosted-toolchain provisioning versus the Git
workflow policy and its fixtures. The second stream incorporated the first
stream's merged base rather than editing a shared worker checkout.

## Observations

| Boundary | Result | Evidence and limitation |
| --- | --- | --- |
| Context isolation | Pass with a harness caveat | Delegated work used separate task contexts and the review pass was independent. The portable registry does not promise parent-context inheritance, so the overlay requires explicit task evidence and skill routing. |
| Worktree ownership | Pass with cleanup out of scope | Worker branches were purpose-first and writable checkouts lived outside the main checkout; the `ci/git-policy` worktree was `/tmp/recite-worktrees/ci-git-policy`. The exercise did not treat later worktree removal as merge evidence. |
| Review | Pass with an external-integration gap | Clean independent diff reviews covered the current heads, and the coordinator checked GitHub threads and protection. `@codex review` on #147 did not produce a standard GitHub review before merge, so #140 remains open; no custom parser was restored. |
| Protected merge | Pass | All three listed PRs merged through protected `main` as signed squash commits after checks; issue state was closed and the resulting main SHAs were verified. |

## Follow-up

The exercise supports retaining this thin overlay. It does not justify a
parallel launcher, a project-local role registry, or assumptions that all
harnesses provide identical native delegation and review behaviour.
