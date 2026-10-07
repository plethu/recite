# Historical evidence

Current contracts, code and contributor guides own implementation and workflow. This directory keeps
raw measurements and selected older designs; results apply only to their recorded revisions and
execution profiles.

| Current question                        | Maintained owner                                                                                                                                               |
| --------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| LSP ownership and performance           | [Architecture](../lsp-cancellation-design.md), [dependency decisions](../lsp-dependency-decisions.md), [profiling procedure](../profiling-and-optimisation.md) |
| Original project-index direction        | [Historical design](lsp-project-index-design.md); its full-sync and issue decomposition are superseded                                                         |
| Migration delivery                      | [Import crate](../../crates/recite-import/README.md); [historical design](adoption-migration-design.md)                                                        |
| Package, engine and editor observations | [Historical delivery evidence](delivery-evidence.md); current guides and repeatable acceptance checks own support claims                                       |

## LSP investigation history

The concluded Markdown reports are preserved at `6e32b614bd8c91a6616f02ec2991b7e300808129` rather
than repeated in the maintained checkout. Their raw assets remain under `lsp-optimisation/`. Use the
exact revision when reading a report; its relative links and commands describe that revision.

| Investigation                          | Historical report                                                                                                                                                       |
| -------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Implementation and baseline            | [history.md](https://github.com/plethu/recite/blob/6e32b614bd8c91a6616f02ec2991b7e300808129/docs/archive/lsp-optimisation/history.md)                                   |
| Editor experience and regression gates | [follow-up.md](https://github.com/plethu/recite/blob/6e32b614bd8c91a6616f02ec2991b7e300808129/docs/archive/lsp-optimisation/follow-up.md)                               |
| Incremental-analysis prototypes        | [prototypes.md](https://github.com/plethu/recite/blob/6e32b614bd8c91a6616f02ec2991b7e300808129/docs/archive/lsp-optimisation/prototypes.md)                             |
| CPU and allocation profiles            | [profiling.md](https://github.com/plethu/recite/blob/6e32b614bd8c91a6616f02ec2991b7e300808129/docs/archive/lsp-optimisation/profiling.md)                               |
| Session and platform testing           | [session-testing.md](https://github.com/plethu/recite/blob/6e32b614bd8c91a6616f02ec2991b7e300808129/docs/archive/lsp-optimisation/session-testing.md)                   |
| Recovery calibration                   | [recovery-calibration.md](https://github.com/plethu/recite/blob/6e32b614bd8c91a6616f02ec2991b7e300808129/docs/archive/lsp-optimisation/recovery-calibration.md)         |
| Native scheduling traces               | [native-tracing.md](https://github.com/plethu/recite/blob/6e32b614bd8c91a6616f02ec2991b7e300808129/docs/archive/lsp-optimisation/native-tracing.md)                     |
| Channel handoff controls               | [channel-handoff.md](https://github.com/plethu/recite/blob/6e32b614bd8c91a6616f02ec2991b7e300808129/docs/archive/lsp-optimisation/channel-handoff.md)                   |
| Worker and response ownership          | [resource-tradeoff.md](https://github.com/plethu/recite/blob/6e32b614bd8c91a6616f02ec2991b7e300808129/docs/archive/lsp-optimisation/resource-tradeoff.md)               |
| Dependency experiments                 | [dependency-decisions.md](https://github.com/plethu/recite/blob/6e32b614bd8c91a6616f02ec2991b7e300808129/docs/archive/lsp-optimisation/dependency-decisions.md)         |
| Transport and resource follow-up       | [continuation.md](https://github.com/plethu/recite/blob/6e32b614bd8c91a6616f02ec2991b7e300808129/docs/archive/lsp-optimisation/continuation.md)                         |
| Final capacity investigation           | [final-resource-profiling.md](https://github.com/plethu/recite/blob/6e32b614bd8c91a6616f02ec2991b7e300808129/docs/archive/lsp-optimisation/final-resource-profiling.md) |
| Retired tools and reproduction         | [retired-probes.md](https://github.com/plethu/recite/blob/6e32b614bd8c91a6616f02ec2991b7e300808129/docs/archive/lsp-optimisation/retired-probes.md)                     |

For example, read a report or export all reports and evidence outside the checkout:

```sh
git show 6e32b614:docs/archive/lsp-optimisation/final-resource-profiling.md
mkdir /tmp/recite-lsp-evidence
git archive 6e32b614 docs/archive/lsp-optimisation | tar -x -C /tmp/recite-lsp-evidence
```

Retired channel/stdio, driver-accounting, stopped-response and macOS-yield tools are preserved with
their tests and workflow inputs at `1004de99594d`. Export that revision for their original commands;
do not restore settled experiments to normal CI. For a changed contract or practical symptom, port
the smallest necessary probe to the current harness.

Keep useful decisions and reopening conditions in their current owner. Concluded narrative belongs
in commit/PR history; retain raw evidence only when it supports a decision or reproduction. Do not
create a report or stub for every retired path.
