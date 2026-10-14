use super::*;
use recite_core::source_lines;

#[test]
fn indexed_selector_lookup_preserves_nearest_header_and_explicit_override() {
    let selectors = [
        MetadataContextSelector::FieldSpeaker,
        MetadataContextSelector::MetadataKey("tone".to_owned()),
    ];
    for newline in ["\n", "\r\n", "\r"] {
        for body in [
            ":: first speaker=mara\n> line tone=calm\n",
            ":: first speaker=mara\n:: nearest\n> line tone=calm",
            ":: first speaker=mara\n:: nearest speaker=\n> line",
            ":: first speaker=mara\n> line speaker= tone=calm",
            ":: first speaker=mara\n> line speaker=[mara] tone=calm",
            ":: first speaker=mara\n> line speaker=other speaker=mara",
            ":: first speaker=mara\n  :: not_a_header speaker=other\n> line",
            "أهلاً😀\n:: first speaker=mara\n> line tone=é\n",
            "",
        ] {
            let text = body.replace('\n', newline);
            let index = SourceLineIndex::new(text.as_str());
            for number in 0..u32::try_from(index.line_count() + 2).expect("line count") {
                for target in [
                    MetadataTarget::Line,
                    MetadataTarget::Choice,
                    MetadataTarget::Block,
                ] {
                    for selector in &selectors {
                        assert_eq!(
                            outcome(resolve_selector(&index, selector, number, target)),
                            outcome(scanned_selector(&text, selector, number, target)),
                            "{text:?} {selector:?} {number} {target:?}",
                        );
                    }
                }
            }
        }
    }
}

fn outcome(value: SelectorResolution<'_>) -> Option<Result<&str, ()>> {
    match value {
        SelectorResolution::Missing => None,
        SelectorResolution::Malformed => Some(Err(())),
        SelectorResolution::Value(value) => Some(Ok(value)),
    }
}

fn scanned_selector<'a>(
    text: &'a str,
    selector: &MetadataContextSelector,
    number: u32,
    target: MetadataTarget,
) -> SelectorResolution<'a> {
    let Some((line, _)) = source_lines(text).nth(number.saturating_sub(1) as usize) else {
        return SelectorResolution::Missing;
    };
    if matches!(selector, MetadataContextSelector::FieldSpeaker)
        && matches!(target, MetadataTarget::Line)
        && !recite_parser::metadata_assignments(line)
            .iter()
            .any(|value| value.key == "speaker")
    {
        let prior = source_lines(text)
            .map(|(content, _)| content)
            .take(number.saturating_sub(1) as usize)
            .collect::<Vec<_>>();
        for line in prior.into_iter().rev() {
            if line.trim_start().starts_with("::") {
                return selector_symbol(line, "speaker");
            }
        }
    }
    match selector {
        MetadataContextSelector::FieldSpeaker if !matches!(target, MetadataTarget::Line) => {
            SelectorResolution::Missing
        }
        MetadataContextSelector::FieldSpeaker => selector_symbol(line, "speaker"),
        MetadataContextSelector::MetadataKey(key) => selector_symbol(line, key),
    }
}
