# Recite v1 Dependency Roadmap

A snapshot of how the open v1 milestones and issues depend on each other: what
can start now, what's blocked, and what's on the critical path.

This is a planning aid. The live Codeberg board and
`docs/recite-production-spec.md` §22–23 are authoritative; the issue numbers and
edges here were pulled from the "Depends on" lines in issue bodies on 2026-05-30,
and will drift as work lands.

## v1 scope

Per spec §23, v1 is broader than "core + CLI + LSP." It requires all of:

- core runtime, CLI, and LSP authoring support;
- a scale and performance proof;
- a stable engine-adapter contract;
- at least one production-quality engine adapter;
- adoption and migration documentation that lets a team evaluate Recite against
  established dialogue tooling.

The release-hardening milestone (M14) can't complete until scale, adapters, and
adoption docs have all landed.

## Work that can start now

These issues have no unmet dependencies. One of them unlocks a whole authoring
track; the rest are independent and can run in parallel.

| Issue | Role | Unlocks |
| --- | --- | --- |
| #137 Metadata-domain design | unblocks a track | metadata value syntax, contextual validation, LSP schema-domain support, adapter schema export |
| #75 Trace performance counters | leaf | — |
| #88 Docs site scaffold | leaf | later docs content |
| #91 Rustdoc API examples | leaf | — |
| #96 Migration transition guides | leaf | — |
| #95 Importer-boundary design | design | #97 |
| #81 Unity adapter design | design | feeds #108, Unity refresh |
| #83 Editor highlighting strategy | design | M12 editor extensions |

## Tracks

Each track below starts from one of the issues above. They all feed the release
milestone.

```mermaid
flowchart LR
  subgraph LSP["LSP track (authoring)"]
    direction LR
    i137["#137 metadata-domain design"] --> i138["#138"]
    i137 --> i139["#139"]
    i138 --> i139
    i139 --> i76["#76"]
    i76 --> i30["#30"]
    i76 --> i31["#31"]
    i76 --> i32["#32"]
    i76 --> i33["#33"]
    i76 --> i77["#77"]
    i76 --> i106["#106"]
    i84["#84"] --> i86["#86"]
  end

  subgraph ADP["Adapter track (critical path)"]
    direction LR
    i140["#140 schema-domain export"] -.-> i79["#79"]
    i140 -.-> i80["#80"]
    i140 -.-> i82["#82"]
    i140 -.-> i107["#107"]
    i79 --> i120["#120"]
    i79 --> i121["#121"]
    i79 --> i122["#122"]
    i120 --> i123["#123"]
    i121 --> i123
    i122 --> i123
    i123 --> i94["#94 adapter docs"]
    i79 --> i108["#108 (also needs #81)"]
    i81["#81"] --> i108
  end

  subgraph PERF["Perf track"]
    direction LR
    i74["#74"]
    i36["#36"]
    i105["#105"]
    i126["#126"]
    i75["#75 counters"] --> i74
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

  subgraph REL["Release hardening (M14)"]
    i112["#112"]
    i113["#113"]
    i114["#114"]
    i115["#115"]
    i116["#116"]
  end

  i137 --> i140
  i85["#85"] -.-> editor["M12 editor extensions, #70 docs"]
  i94 -- adapters --> REL
  i74 -- scale proof --> REL
  DLATER -- adoption docs --> REL
```

## Critical path

The adapter chain is still the longest release chain, but the original adapter
contract design issue (#78) is now closed. The current open chain is:

```
#79 conformance + per-engine adapter MVPs → per-engine refresh workflows → adapter docs → release verification
```

The new metadata-domain design gate (#137) also matters for both authoring and
adapters. It feeds the LSP schema/index work (#76, #30, #31, #77) and the
resource-backed schema-domain export contract (#140), so it should be resolved
before treating schema-aware completions, diagnostics, or adapter schema export
as routine implementation.

## Release hardening

The M14 "Release:" issues (#112–#116) sit at the end of the graph. They can't
start until scale, adapters, and adoption docs are in place.
