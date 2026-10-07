# Retired LSP diagnostic tools

Historical evidence archived from `docs/design/lsp-cancellation/retired-probes.md` at `58b8f04965af`.
This records its named revision and execution profile; it is not current
workflow or implementation authority. Historical commands use their original
revision and paths. See the [archive index](../README.md).

The tools and CI modes below answered bounded questions during #206. Their
results and rejected/retained decisions remain in the linked reports. The final
maintenance pass removes them from active tooling rather than turning every
experiment into a supported command.

| Retired tool/control | Decision evidence |
| --- | --- |
| Channel handoff and raw stdio microprobes | [Channel handoff](channel-handoff.md), [resource tradeoff](resource-tradeoff.md) |
| Editing CPU sampler A/B and startup idle relocation | [Resource tradeoff](resource-tradeoff.md) |
| Prepared stopped-response A/B | [Resource tradeoff](resource-tradeoff.md) |
| Historical Python driver comparison, switch interval and macOS yield override | [Recovery calibration](recovery-calibration.md), [native tracing](native-tracing.md) |
| Standalone rename fanout script | Maintained fanout generation and the paired process gate cover the recurring comparison |

All tools, private tests, Cargo targets and workflow inputs are preserved at
`1004de99594d` (the complete pre-cleanup repository revision). Reproduce a
historical command from an isolated export of that revision, using its own setup
and the control/candidate revisions stated in the report:

```sh
mkdir /tmp/recite-retired-lsp-probes
# Run from the Recite checkout; the destination must be outside it.
git archive 1004de99594d | tar -x -C /tmp/recite-retired-lsp-probes
```

Reports that mention removed workflow inputs or script names describe that
historical revision. Do not run them against current tooling or restore them to
normal CI merely to repeat a settled decision. Reopen the experiment only for a
changed contract or a new practical symptom; port the smallest required probe to
the current harness and archive it again when the decision is made.

The ordinary session matrix, final-handoff cancellation tests, paired regression
gate, native tracing, server comparison and resource/latency fault injection stay
maintained. Report schemas, phase boundaries and acceptance budgets are preserved
by the tooling cleanup.
