use super::super::diagnostics::{PoDiagnostic, PoPluralDiagnostic, error};
use super::super::syntax::parse_quoted;
use super::super::types::PoFieldTarget;

pub(crate) fn directive(
    name: &str,
    line: usize,
    input: &str,
) -> Result<Option<(String, String, PoFieldTarget)>, super::super::PoParseError> {
    let Some(index) = input.find(char::is_whitespace) else {
        return Ok(None);
    };
    let keyword = &input[..index];
    let value_input = input[index..].trim_start();
    if !value_input.starts_with('"') {
        return Ok(None);
    }
    let value = parse_quoted(name, line, value_input)?;
    let target = match keyword {
        "msgctxt" => PoFieldTarget::Context,
        "msgid" => PoFieldTarget::SourceText,
        "msgid_plural" => PoFieldTarget::PluralSourceText,
        "msgstr" => PoFieldTarget::Translation,
        _ if keyword.starts_with("msgstr[") => {
            let index = keyword
                .strip_prefix("msgstr[")
                .and_then(|value| value.strip_suffix(']'))
                .and_then(|value| value.parse().ok())
                .ok_or_else(|| {
                    error(
                        name,
                        line,
                        PoDiagnostic::InvalidPluralArms(PoPluralDiagnostic::InvalidArm(
                            keyword.to_owned(),
                        )),
                    )
                })?;
            PoFieldTarget::PluralTranslation(index)
        }
        _ => PoFieldTarget::Unknown,
    };
    Ok(Some((keyword.to_owned(), value, target)))
}
