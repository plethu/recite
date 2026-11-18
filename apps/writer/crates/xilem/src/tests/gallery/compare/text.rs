//! Text size against the baseline: a control's or a label's text never shrinks at the same frame.
use super::Report;
use serde_json::Value;
use std::collections::BTreeMap;

/// Font sizes may differ by rounding; anything more is a shrink.
const SIZE_TOLERANCE: f64 = 0.5;
/// Without a recorded font size the baseline's line box stands in for it, and a line box also
/// holds leading, which a design may tighten: a line box may lose this fraction before it counts.
/// Such a finding cannot tell type from leading, so a reviewer justifies it; once both frames
/// record font sizes, any shrink blocks.
const LINE_TOLERANCE: f64 = 0.1;

type Key = (String, String);

/// Interactive controls by role and name, and text labels by their words, in reading order.
fn texts(snapshot: &Value) -> BTreeMap<Key, Vec<&Value>> {
    let mut texts: BTreeMap<Key, Vec<&Value>> = BTreeMap::new();
    for node in snapshot["nodes"].as_array().into_iter().flatten() {
        let role = node["role"].as_str().unwrap_or("");
        let interactive = node["interactive"].as_bool() == Some(true)
            && !matches!(role, "ScrollView" | "ScrollBar");
        let words = node["label"].as_str().or_else(|| node["value"].as_str());
        let Some(words) = words.filter(|words| !words.trim().is_empty()) else {
            continue;
        };
        if interactive || role == "Label" {
            texts
                .entry((role.to_owned(), words.to_owned()))
                .or_default()
                .push(node);
        }
    }
    texts
}

fn height(node: &Value) -> Option<f64> {
    let bounds = node["bounds"].as_array()?;
    Some(bounds.get(3)?.as_f64()? - bounds.get(1)?.as_f64()?)
}

/// How much less text `new` sets than `old`, and whether that is certain: font sizes where both
/// snapshots record them, else a text label's line box, its height shared among the lines the
/// new line box would make.
fn shrank(old: &Value, new: &Value, label: bool) -> Option<(String, bool)> {
    let [size, line] = [0, 1].map(|index| new["text"][index].as_f64());
    let (size, line) = (size?, line?);
    if let Some(before) = old["text"][0].as_f64() {
        return (size < before - SIZE_TOLERANCE).then(|| (format!("{before}px to {size}px"), true));
    }
    if !label || line <= 0. {
        return None;
    }
    let total = height(old)?;
    let before = total / (total / line).round().max(1.);
    (line < before * (1. - LINE_TOLERANCE))
        .then(|| (format!("a {before}px line to {line}px"), false))
}

/// Block each control or label whose text is smaller than in the baseline frame, and list for
/// review each label whose line box alone says so.
pub(super) fn shrinks(name: &str, (before, after): (&Value, &Value), report: &mut Report) {
    let after = texts(after);
    for ((role, words), nodes) in texts(before) {
        let Some(current) = after.get(&(role.clone(), words.clone())) else {
            continue;
        };
        for (old, new) in nodes.iter().zip(current) {
            if let Some((change, certain)) = shrank(old, new, role == "Label") {
                let finding = format!("{name}: {role} {words:?} text shrank from {change}");
                if certain {
                    report.regressions.push(finding);
                } else {
                    report.review.push(finding);
                }
            }
        }
    }
}
