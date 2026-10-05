# Recovery attribution and driver pacing

The first cross-platform session run showed much higher burst recovery on the
macOS runner. A phase-attribution experiment kept the release server unchanged
and repeated each workload three times per operating system. It measured elapsed
protocol stages, driver edit construction, trace writes, requested sleeps, and
process CPU time separately.

[Attribution run 37375835810](https://github.com/plethu/recite/actions/runs/37375835810)
used harness revision `a201f97d`. Medians below combine the fixed and churn runs,
excluding the first five cycles of each repetition. Driver costs overlap the
protocol stages; these columns are not additive.

| Measurement per cycle | Linux | macOS | Windows |
| --- | ---: | ---: | ---: |
| Time in 50 requested 5 ms sleeps | 254 ms | 1,309 ms | 263 ms |
| Final edit to start of request draining | 5.1 ms | 25.5 ms | 5.2 ms |
| Repair to diagnostics, including edit construction | 5.4 ms | 6.0 ms | 13.7 ms |
| Driver CPU | 197 ms | 168 ms | 219 ms |
| Server CPU | 500 ms | 530 ms | 1,063 ms |

The macOS runner's oversleep accounted for most of the elapsed workload gap and
much of the reported recovery gap. The similar Linux/macOS server CPU costs and
repair timings do not support a broad server CPU slowdown. They do not eliminate
tail variability: macOS's cancellation-response p95 was about 20 ms in this run.
[GitHub's runner hardware](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)
differs between platforms, so these are observations from the experiment rather
than a ranking of operating systems.

The driver now schedules edits against successive 5 ms deadlines and begins
draining immediately after the final edit. Oversleep can still produce late or
grouped edits, but it no longer shifts every subsequent deadline. Reports retain
actual send intervals and the first-to-last send span. No busy wait or OS-specific
scheduling override is used. This is a measurement correction; production LSP
code did not change.

The optional control harness uses the same release binary and fixture as the
candidate. Three repetitions alternate control/candidate ordering. To reproduce
the driver comparison:

```sh
gh workflow run ci.yml --repo plethu/recite --ref BRANCH \
  -f lsp_sessions_only=true \
  -f lsp_driver_control_ref=a201f97da83c26b4e4ba53875b4ae23d4295992b
```

The control checkout is isolated from build caches. Ordinary CI runs omit the
control revision and execute only the current driver.

## Paired pacing results

[Run 37377620058](https://github.com/plethu/recite/actions/runs/37377620058)
compared the original driver with revision `4460a998` on the same binary and
fixture within each job. Each side ran six workloads, 240 cycles and 12,000 edits.
The table reports pooled post-warmup recovery medians and the range of the six
workload p95 values. These are changes in the exercised workload and measurement,
not production server speedups.

| Host | Recovery median, original → paced | Workload p95, original → paced | Total workload time, original → paced |
| --- | ---: | ---: | ---: |
| Linux | 11.68 → 6.68 ms | 11.8–12.9 → 8.0–8.7 ms | 199.0 → 162.4 s |
| macOS | 36.77 → 9.76 ms | 55.9–79.3 → 11.8–66.9 ms | 458.2 → 154.2 s |
| Windows | 17.33 → 10.43 ms | 20.1–23.2 → 12.0–14.2 ms | 227.8 → 191.6 s |

The median macOS first-to-last edit span became 252 ms for a requested 245 ms.
The final-edit-to-drain delay fell from 23.74 ms to 0.05 ms. Its tails remain
variable between repetitions, so the median improvement alone is insufficient
for choosing a tight tail budget.

## Rejected Python scheduling experiment

[Run 37380032116](https://github.com/plethu/recite/actions/runs/37380032116)
compared the corrected driver at the interpreter's default switch interval with
a 1 ms interval. It also recorded the interval between full response receipt in
the reader and resumption of the main driver. This interval includes JSON
parsing and queue handoff; it does not measure scheduling before the read.

The 1 ms setting did not improve macOS: median recovery changed from 20.30 ms to
30.85 ms, and driver CPU increased from 169.15 ms to 176.81 ms per cycle. Linux
and Windows changed little. MacOS reader-to-main p95 was 0.22 ms for repair,
0.13 ms for cancelled rename, and 0.14 ms for completion. The shorter interval
is rejected; ordinary runs retain the interpreter default. The optional
`--driver-switch-ms 1` flag retains a way to reproduce the experiment, and every
report records the effective interval.

Inspection of the server event loop and workers found no fixed recovery timer.
The pinned `lsp-server` 0.7.9 stdio transport uses zero-capacity channels; the
pinned Crossbeam backoff can yield the thread before blocking. This identifies
possible scheduling boundaries, not the cause of the remaining 10–20 ms steps.
Changing channel buffering without attribution would also change the handoff
boundary used for cancellation and publication freshness. No transport change
is retained. Native timestamps around input, dispatch, worker completion and
writer handoff would be the next targeted experiment if those tails warrant
further work; current evidence cannot distinguish server waits from scheduling
before the driver's response read.

## Enforced recovery budgets

CI enforces post-warmup p95 limits of 20 ms on Linux, 40 ms on Windows and 75 ms
on macOS, independently for fixed and churn workloads. At least two of three
repetitions above the limit fail the job; every cycle still has an unconditional
500 ms recovery bound. These platform budgets leave headroom over the observed
repeated tails, especially macOS variability. They detect substantial tail
regressions, not every slowdown; the existing paired candidate/base gate remains
responsible for relative regressions.

The gate rejects missing, truncated, failed, non-finite or incomparable reports.
Unit tests prove repeated overruns fail and an isolated bad repetition does not.
A CLI negative control against real Linux reports with a 5 ms budget exited 1.
All three hosted jobs in run 37380032116 passed the new gate, live resource fault
controls, incremental protocol tests and the rendered editor lifecycle probe.
The final default-interval validation is recorded below.

[Retained measurement summary](recovery-results.json) records both paired
experiments, revisions, binary and fixture identities, phase timings and editor
probe counts. Raw reports and traces remain in the linked Actions artifacts for
30 days.


## Final validation

[Run 37381950696](https://github.com/plethu/recite/actions/runs/37381950696)
tested revision `069d1a37` with the default Python interval restored. All three
platform jobs and `required-check` passed. Each platform completed six workloads
(240 cycles and 12,000 burst edits), incremental protocol checks, live fault
controls, and the rendered VS Code lifecycle check.

| Host | Recovery median | Range of six workload p95s | CI p95 budget |
| --- | ---: | ---: | ---: |
| Linux | 6.60 ms | 7.90–8.44 ms | 20 ms |
| macOS | 12.82 ms | 15.23–56.01 ms | 75 ms |
| Windows | 15.55 ms | 18.72–22.27 ms | 40 ms |

No repetition exceeded its platform budget. This final run is independent of the
paired experiments above and should not be used as a same-host before/after
comparison. Production Rust code remained unchanged during this refinement.
