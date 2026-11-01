"""Measure Writer diagnostics during the native journeys without reading trace contents."""


def metrics(output):
    logs = list(output.parent.glob(output.stem + "*.writer.log"))
    return {
        "writer_console_bytes": sum(log.stat().st_size for log in logs),
        "automatic_dense_trace_files": sum(
            "Writing full logs to " in log.read_text(errors="replace") for log in logs
        ),
    }
