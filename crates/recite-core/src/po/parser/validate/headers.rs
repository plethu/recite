use super::super::PoEntry;
use super::super::diagnostics::{PoDiagnostic, PoHeaderDiagnostic, error};
use super::super::plural::PluralRuleError;
use super::super::types::PoHeader;

pub(crate) fn parse_headers(
    name: &str,
    entry: &PoEntry,
    value: &str,
) -> Result<Vec<PoHeader>, super::super::PoParseError> {
    let mut headers = Vec::new();
    for line in value.split('\n').map(|line| line.trim_end_matches('\r')) {
        if line.is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            return Err(error(
                name,
                entry.start_line,
                PoDiagnostic::InvalidHeader(PoHeaderDiagnostic::MissingColon(line.to_owned())),
            ));
        };
        let key = key.trim();
        if key.is_empty()
            || headers
                .iter()
                .any(|header: &PoHeader| header.key.eq_ignore_ascii_case(key))
        {
            return Err(error(
                name,
                entry.start_line,
                PoDiagnostic::InvalidHeader(PoHeaderDiagnostic::DuplicateOrEmpty(key.to_owned())),
            ));
        }
        headers.push(PoHeader {
            key: key.to_owned(),
            value: value.trim().to_owned(),
        });
    }
    if let Some(header) = headers
        .iter()
        .find(|header| header.key.eq_ignore_ascii_case("Plural-Forms"))
        && let Err(reason) = super::super::plural::validate_header(&header.value)
    {
        let cause = match reason {
            PluralRuleError::InvalidHeader => PoHeaderDiagnostic::InvalidPluralForms,
            reason => PoHeaderDiagnostic::InvalidPluralRule(reason),
        };
        return Err(error(
            name,
            entry.start_line,
            PoDiagnostic::InvalidHeader(cause),
        ));
    }
    Ok(headers)
}

pub(crate) fn parse_plural_forms(value: &str) -> Option<(usize, String)> {
    super::super::plural::validate_header(value).ok()
}
