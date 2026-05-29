# Recite v1 Dependency Roadmap

A planning snapshot of how the open v1 milestones and issues depend on each
other — what can start now, what's blocked, and what sits on the critical path.

It's a planning aid, not an authority. The live Codeberg board and
`docs/recite-production-spec.md` §22–23 are authoritative; the issue numbers and
edges here are a snapshot (2026-05-29) pulled from the "Depends on" lines in issue
bodies, and will drift as work lands.

## v1 scope

Per spec §23, the v1 boundary is deliberately broad — not just "core + CLI + LSP."
It requires all of:

- core runtime, CLI, and LSP authoring support;
- a scale and performance proof;
- a stable engine-adapter contract;
- at least one production-quality engine adapter;
- adoption and migration documentation that lets a team evaluate Recite against
  established dialogue tooling.

The release-hardening milestone (M14) is the join point: it cannot complete until
scale, adapters, and adoption docs have landed.

## Start-here frontier

Work with no unmet dependencies. Two are keystones that unlock large subtrees;
the rest are independent and can run in parallel.

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

Each track is a subtree rooted in one of the frontier issues above. The release
milestone is the sink they all drain into.

```mermaid
flowchart LR
  subgraph LSP["LSP track (authoring)"]
    direction LR
    i29["#29 scaffold"] --> i76["#76"]
    i29 --> i84["#84"]
    i29 --> i85["#85"]
    i76 --> i30["#30"]
    i76 --> i31["#31"]
    i76 --> i32["#32"]
    i76 --> i33["#33"]
    i76 --> i77["#77"]
    i76 --> i106["#106"]
    i84 --> i86["#86"]
  end

  subgraph ADP["Adapter track (critical path)"]
    direction LR
    i78["#78 contract"] --> i79["#79"]
    i78 --> i80["#80"]
    i78 --> i82["#82"]
    i78 --> i107["#107"]
    i79 --> i120["#120"]
    i79 --> i121["#121"]
    i79 --> i122["#122"]
    i120 --> i123["#123"]
    i121 --> i123
    i122 --> i123
    i123 --> i94["#94 adapter docs"]
    i78 --> i108["#108 (also needs #79, #81)"]
    i81["#81"] --> i108
  end

  subgraph PERF["Perf track"]
    direction LR
    i73["#73 benchmarks"] --> i74["#74"]
    i75["#75 counters"] --> i74
    i36["#36"]
    i105["#105"]
    i126["#126"]
  end

  subgraph DOCS["Docs / adoption"]
    direction LR
    subgraph DNOW["available now"]
      i88["#88"]
      i91["#91"]
      i96["#96"]
    end
    subgraph DLATER["after scale + adapters"]
      i89["#89"]
      i90["#90"]
      i92["#92"]
      i93["#93"]
    end
    DNOW --> DLATER
  end

  subgraph MIG["Migration"]
    i95["#95"] --> i97["#97"]
  end

  subgraph REL["Release hardening — M14 sink"]
    i112["#112"]
    i113["#113"]
    i114["#114"]
    i115["#115"]
    i116["#116"]
  end

  i85 -.-> editor["M12 editor extensions, #70 docs"]
  i94 -- adapters --> REL
  i74 -- scale proof --> REL
  DLATER -- adoption docs --> REL
```

## Critical path

The longest pole is the adapter chain:

```
#78 → per-engine adapter MVPs → per-engine refresh workflows → adapter docs → release verification
```

So `#78` is the place to start. It's independent of the LSP keystone (`#29`), so
the two can progress in parallel.

## Release hardening is a sink

The M14 "Release:" issues (#112–#116) sit at the very end of the graph and can't
start until scale, adapters, and adoption docs are in place — finish-line work,
not a starting point.
