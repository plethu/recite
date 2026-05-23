use crate::{
    Diagnostic, DiagnosticCode, DiagnosticSeverity, SourcePosition, SourceSpan,
    source_location::{point_one, position_for_byte_offset, source_position},
};

#[must_use]
pub fn project_scene_key_span(
    file: &str,
    source: &str,
    scene_index: usize,
    key: &str,
) -> SourceSpan {
    scene_key_span(file, source, scene_index, key)
}

pub(super) fn diagnostic(code: &str, message: impl Into<String>, span: SourceSpan) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::new(code).expect("project diagnostic codes are static and namespaced"),
        DiagnosticSeverity::Error,
        message,
        span,
    )
}

pub(super) fn toml_error_span(file: &str, source: &str, error: &toml::de::Error) -> SourceSpan {
    let Some(span) = error.span() else {
        return manifest_span(file);
    };
    SourceSpan::point(
        file.to_owned(),
        position_for_byte_offset(source, span.start),
    )
}

pub(super) fn scene_key_span(
    file: &str,
    source: &str,
    scene_index: usize,
    key: &str,
) -> SourceSpan {
    scene_key_position(source, scene_index, key)
        .or_else(|| scene_header_position(source, scene_index))
        .map_or_else(
            || manifest_span(file),
            |position| SourceSpan::point(file, position),
        )
}

fn manifest_span(file: &str) -> SourceSpan {
    SourceSpan::point(file.to_owned(), point_one())
}

fn scene_key_position(source: &str, scene_index: usize, key: &str) -> Option<SourcePosition> {
    let mut current_scene = None;
    for (line_index, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("[[scenes]]") {
            current_scene = Some(current_scene.map_or(0, |index| index + 1));
            continue;
        }

        if current_scene == Some(scene_index) && trimmed.starts_with(key) {
            let column = line.find(key).unwrap_or(0) + 1;
            return source_position(line_index + 1, column);
        }
    }

    None
}

fn scene_header_position(source: &str, scene_index: usize) -> Option<SourcePosition> {
    let mut current_scene = 0;
    for (line_index, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("[[scenes]]") {
            if current_scene == scene_index {
                let column = line.find("[[scenes]]").unwrap_or(0) + 1;
                return source_position(line_index + 1, column);
            }
            current_scene += 1;
        }
    }

    None
}
