# Recite v1 Dependency Roadmap

A planning snapshot of how the open v1 milestones and issues depend on each
other. It exists to turn the issue backlog into a workable order: which work can
start now, which work is gated, and what sits on the critical path.

This is a planning aid, not authority. The live Codeberg board and
`docs/recite-production-spec.md` §22–23 are authoritative; issue numbers and edges
here are a snapshot (2026-05-29) derived from the "Depends on" lines in issue
bodies, and will drift as work lands.

## v1 scope

Per spec §23, the serious v1 boundary is broad. It is not "core + CLI + LSP." A
credible v1 requires all of:

- core runtime, CLI, and LSP authoring support;
- a scale and performance proof;
- a stable engine-adapter contract;
- at least one production-quality engine adapter;
- adoption and migration documentation that lets a team evaluate Recite against
  established dialogue tooling.

The release-hardening milestone (M14) is the join point: it cannot complete until
scale, adapters, and adoption docs have landed.

## Start-here frontier

Work with no unmet dependencies. Two of these are keystones that unlock large
subtrees; the rest are independent and can run in parallel (including as
delegated work).

| Issue | Role | Unlocks |
| --- | --- | --- |
| #29 LSP scaffold | **keystone** | the LSP subtree (indexes, diagnostics, navigation, code actions, editor clients) |
| #78 Adapter contract design | **keystone** | the adapter subtree (conformance, per-engine MVPs, refresh workflows, adapter docs) |
| #73 Criterion benchmark suite | ready | perf subtree (its only dependency, the fixture generator, is closed) |
| #75 Trace performance counters | leaf | — |
| #88 Docs site scaffold | leaf | later docs content |
| #91 Rustdoc API examples | leaf | — |
| #96 Migration transition guides | leaf | — |
| #95 Importer-boundary design | design | #97 |
| #81 Unity adapter design | design | feeds #108, Unity refresh |
| #83 Editor highlighting strategy | design | M12 editor extensions |

## Tracks

```
LSP track          #29 ─┬─ #76 ─┬─ #30 #31 #32 #33 #77
(authoring)             │        └─ #106
                        ├─ #84 ─── #86
                        └─ #85
                                          ↘ feeds M12 editor extensions, #70 docs

Adapter track      #78 ─┬─ #79 ─┬─ #120 #121 #122 ── #123 ─┐
(critical path)    #81 ┐│─ #80  │                          ├─ #94
                      └┼─ #82   │                          │
                       ├─ #107  │                          │
                       └─ #108 ←┘ (needs #78, #79, #81)     │
                                                            ↓
Perf track         #73 ─┬─ #74  #36  #126                      (work joins
                   #75 ─┘       #105                            at release)
                                                            ↓
Docs / adoption    #88 #91 #96 (now) → #89 #90 #92 #93 (after scale + adapter)
Migration          #95 → #97
                                                            ↓
Release (M14)      #112 #113 #114 #115 #116  ← gated on scale + adapters + docs
```

## Critical path

The longest pole is the adapter chain:

```
#78 → per-engine adapter MVPs → per-engine refresh workflows → adapter docs → release verification
```

`#78` is therefore the single highest-leverage place to start. It is independent
of the LSP keystone (`#29`), so the two can progress in parallel.

## Release hardening is a sink

The M14 "Release:" issues (#112–#116) sit at the very end of the graph and cannot
start until scale, adapters, and adoption docs are in place. They are the finish
line, not early work.
