# Bounded LSP latency experiments

The verified cancellation and region-cache checkpoint is `6abcb759`. Experiments
below are local Linux stdio measurements, not editor frame timings. No builds,
tests or samplers ran during normal-release timing comparisons. Each edit study
uses three alternating pairs, two warmups and 21 recorded samples per workload
per process. Diagnostic hashes must match the control; valid edits must publish
no diagnostics. Power state is recorded per run.

## A: Move fresh region outputs — rejected

Question: can consuming fresh/relocated region arrays, while borrowing unchanged
ones, reduce newline and cold-index cost without slowing ordinary edits?

This disposable prototype changed only four compiler composition files. It kept
exact-equality and shared-slice reuse, moved owned summary/fact/diagnostic arrays,
and reserved final capacities. The exact patch against `6abcb759` is retained as
`move-output-prototype.patch`; production code was restored after measurement.

| Large workload | Control median | Prototype median |
| --- | ---: | ---: |
| Prose | 3.13 ms | 3.29 ms |
| Newline | 7.10 ms | 6.48 ms |
| Stable-ID label | 4.66 ms | 5.53 ms |
| New block | 14.02 ms | 14.59 ms |
| Recovery transition | 21.06 ms | 21.34 ms |
| Initial index, five alternating process pairs | 385.59 ms | 371.94 ms |

Raw observations are `move-edit-paired.json` and `move-cold-paired.json`. Cold
here means a new server process; filesystem caches remain warm. The first
control startup was 530 ms, so use the medians rather than that outlier.

The compiler suite and Clippy passed; diagnostic hashes matched; both unchanged
writer heap budgets passed. `move-heap.json` retains allocation evidence. Linked
writer peak live allocation fell from 15.70 MB to 14.98 MB. The newline/index
and allocation savings do not justify the roughly 19% stable-ID latency
regression and added ownership plumbing. Keep the current simpler composition
until a more compact representation proves a broader benefit.
