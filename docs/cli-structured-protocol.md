# Structured CLI protocol

Recite's non-interactive commands support an opt-in machine-output
boundary:

```text
recite validate --output-format structured [--invocation-id ID] PATHS...
recite compile --output OUTPUT --output-format structured [--invocation-id ID] PATHS...
recite extract --output-format structured [--output OUTPUT] [--invocation-id ID] PATHS...
recite run --output-format structured [--invocation-id ID] ASSET --block BLOCK --fixture FIXTURE
recite trace --output-format structured [--invocation-id ID] ASSET --block BLOCK --fixture FIXTURE
recite watch --output-format structured [--invocation-id ID] PROJECT-ROOT
```

The default output format remains the existing human CLI surface. `watch` is a
streaming lifecycle and does not use the finite two-record result contract.

## Version 1 envelope

Structured output is newline-delimited JSON on stdout. Every record is written
and flushed before the next record. A successful invocation, including one that
finds invalid source content, has exactly two records:

1. `command.started`, sequence `0`;
2. one terminal `command.result`, sequence `1`.

An operational failure has the same first record and a terminal
`command.error`. Sequences are zero-based and increase monotonically. Every
record has `version: 1`, `event`, and `command`. When supplied, the opaque
caller-owned `invocation_id` is copied to both records. The protocol never
includes a process ID.

`command.result` has a typed command data object, a status, and an exit code:

- `success` and exit `0` mean the command completed successfully;
- `content_diagnostics` and exit `1` mean source or schema content was
  understood but failed validation. Diagnostics remain in the result data.

Clap syntax and argument errors happen before the structured protocol starts.
They retain Clap's existing process and presentation behavior.

## Result data

The data object is selected by command, and its shape is coupled to the result
status. A successful `validate` contains typed `diagnostics`. A successful
`compile` contains diagnostics and required `artifact` metadata; a compile
with content diagnostics contains diagnostics and no artifact. A successful
`extract` contains diagnostics and exactly one of artifact metadata (when
`--output` is supplied) or typed `entries`; an extract with content diagnostics
contains diagnostics only. `run` and `trace` return the deterministic runtime
`trace` model; they do not return localized human run lines. The protocol does
not add redundant `valid`, `compiled`, or `extracted` booleans: `status` and
the command-specific data shape identify the outcome phase.

Artifact metadata contains an exact machine path projection and `size_bytes`.
It never contains artifact bytes. Paths use `utf8` when representable and an
exact platform byte or wide-unit encoding when they are not.

Diagnostics use the locale-neutral `DiagnosticRecord` representation, including
stable codes and source spans. Human compatibility messages are not used to
drive machine decisions.

## Operational errors

`command.error` has `status: "failure"`, exit `1`, and an error object with
stable `category`, `code`, and `operation` fields. Where applicable it also
contains exact `path`, `related_path`, and typed `details`. Categories
distinguish input, I/O, schema, compilation, asset, fixture, runtime,
localisation, configuration, serialization, and other stable failure classes.
Errors are mapped from typed CLI failures rather than parsed display text.

Structured mode writes protocol records only to stdout and does not leak human
diagnostics or localized run output to stderr. A broken output pipe is handled
as the normal CLI write failure; callers should treat an incomplete stream as
non-conforming rather than attempting recovery from a partial record.

## Structured `watch`

`watch` emits version-1 NDJSON records, flushing each record before continuing:

1. `watch.started`;
2. for each build attempt, `watch.build.started` (with `trigger: "initial"` or
   `trigger: "input_changed"`) followed by exactly one
   `watch.build.completed`;
3. `watch.waiting` after every non-terminal attempt;
4. `watch.cancel.requested` when a valid cancellation control is received; and
5. `watch.stopped` when the process exits.

Build completion data is an explicit CLI projection. It contains the build
`generation` and optional `snapshot_generation` (null for preparation-only
diagnostics), sorted, deduplicated project-relative `inputs`, locale-neutral
`diagnostics`, exact machine path plus `size_bytes` for published `artifacts`, typed
`publication`, `recovery`, `freshness`, and `cancellation` values, and
`restart_guidance: {"type":"host_policy_required","decision":"unspecified"}`.
Compiler fingerprints, candidate bytes, process IDs, wall-clock values, and
telemetry are not wire data. Human watch output and debounce/generated-output
filtering are unchanged.

Preparation-only diagnostics use `snapshot_generation: null`,
`publication: {"type":"not_attempted","reason":"preparation_failed"}`, and
the sorted inputs known before preparation. Recoverable preparation or host
build failures use the tagged `operational_failure` outcome and retain their
typed error; they do not claim that publication occurred. A post-publication
freshness failure is tagged `freshness_failure` while retaining the published
outcome and any recovery records. Unknown future lifecycle or publication
variants remain explicitly tagged `unknown` rather than being treated as a
successful or failed publication.

The process-scoped stdin control transport is also versioned NDJSON. A caller
may request cancellation with:

```json
{"version":1,"command":"watch","action":"cancel"}
```

When the command was started with `--invocation-id`, the control may include a
matching `invocation_id`; a mismatched control is reported as a typed,
recoverable `watch.control.error`. Malformed, unsupported-version, unsupported
command/action, and mismatched controls are recoverable and do not stop the
watch. EOF is not cancellation. A valid cancellation wakes an idle watcher
and cooperatively cancels an active build through the shared build control.
Notify failures are similarly reported as typed recoverable records. Startup,
stream, and fatal watch failures are typed in the terminal `watch.stopped`
record. No OS signal transport is implied.
