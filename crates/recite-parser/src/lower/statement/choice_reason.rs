use crate::header::HeaderKeyValue;
use crate::source::span_for_text;
use recite_core::SourceSpan;

pub(super) struct ParsedReasonOverride<'a> {
    pub(super) id: &'a str,
    pub(super) id_span: SourceSpan,
    pub(super) argument_span: Option<SourceSpan>,
}

pub(super) fn parse_reason_override_value<'a>(
    path: &str,
    kv: &HeaderKeyValue<'a>,
) -> Option<ParsedReasonOverride<'a>> {
    if let Some(open) = kv.value.find('(') {
        let close = kv.value.rfind(')')?;
        if close != kv.value.len() - 1 || open == 0 {
            return None;
        }
        let id = &kv.value[..open];
        let argument_text = &kv.value[open..=close];
        let value_column = source_column(kv.value_span.start.column());
        let id_span = span_for_text(path, kv.value_span.start.line(), value_column, id);
        let argument_span = span_for_text(
            path,
            kv.value_span.start.line(),
            value_column + id.chars().count(),
            argument_text,
        );
        return Some(ParsedReasonOverride {
            id,
            id_span,
            argument_span: Some(argument_span),
        });
    }
    if kv.value.contains(')') {
        return None;
    }
    Some(ParsedReasonOverride {
        id: kv.value,
        id_span: kv.value_span.clone(),
        argument_span: None,
    })
}

pub(super) const fn source_column(column: u32) -> usize {
    column as usize
}
