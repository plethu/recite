# Structured CLI protocol

Recite's five non-interactive commands support an opt-in machine-output
boundary:

```text
recite validate --output-format structured [--invocation-id ID] PATHS...
recite compile --output OUTPUT --output-format structured [--invocation-id ID] PATHS...
recite extract --output-format structured [--output OUTPUT] [--invocation-id ID] PATHS...
recite run --output-format structured [--invocation-id ID] ASSET --block BLOCK --fixture FIXTURE
recite trace --output-format structured [--invocation-id ID] ASSET --block BLOCK --fixture FIXTURE
```

`watch` is not part of this protocol. The default output format remains the
existing human CLI surface.

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
