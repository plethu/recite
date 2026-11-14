//! Compare a fresh capture with the signed-off baseline: pixels for review, accessibility for gates.
use super::record::MINIMUM_TARGET;
use image::{Rgba, RgbaImage};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

/// A channel difference at or below this is antialiasing noise, not a change.
const TOLERANCE: u8 = 8;

#[derive(Default)]
pub(crate) struct Report {
    pub unchanged: usize,
    pub changed: Vec<(String, f64)>,
    pub missing: Vec<String>,
    pub added: Vec<String>,
    /// Accessibility losses against the baseline. Each one blocks sign-off until fixed.
    pub regressions: Vec<String>,
    /// Accessibility differences a reviewer must justify, such as a renamed control.
    pub review: Vec<String>,
}

fn frames(manifest: &Value) -> BTreeMap<String, (String, Value)> {
    let mut frames = BTreeMap::new();
    for entry in manifest["entries"].as_array().into_iter().flatten() {
        let id = entry["id"].as_str().unwrap_or("").to_owned();
        for frame in entry["frames"].as_array().into_iter().flatten() {
            let name = frame["name"].as_str().unwrap_or("").to_owned();
            frames.insert(name, (id.clone(), frame.clone()));
        }
    }
    frames
}

pub(crate) fn compare(current: &Path, manifest: &Value, baseline: &Path) -> Option<Report> {
    let saved: Value =
        serde_json::from_str(&fs::read_to_string(baseline.join("manifest.json")).ok()?).ok()?;
    let ran: BTreeSet<String> = manifest["entries"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|entry| entry["id"].as_str().map(str::to_owned))
        .collect();
    let (before, after) = (frames(&saved), frames(manifest));
    let diffs = current.join("diff");
    if diffs.exists() {
        let _ = fs::remove_dir_all(&diffs);
    }
    let _ = fs::create_dir_all(&diffs);
    let mut report = Report::default();
    for (name, (entry, old)) in &before {
        if !ran.contains(entry) {
            continue;
        }
        let Some((_, new)) = after.get(name) else {
            report.missing.push(name.clone());
            continue;
        };
        let file = |root: &Path, frame: &Value| root.join(frame["file"].as_str().unwrap_or(""));
        match pixels(
            &file(baseline, old),
            &file(current, new),
            &diffs.join(format!("{name}.webp")),
        ) {
            Some(0.) => report.unchanged += 1,
            Some(ratio) => report.changed.push((name.clone(), ratio)),
            None => report.changed.push((name.clone(), 1.)),
        }
        let read = |root: &Path, frame: &Value| -> Value {
            fs::read_to_string(root.join(frame["a11y"].as_str().unwrap_or("")))
                .ok()
                .and_then(|text| serde_json::from_str(&text).ok())
                .unwrap_or(Value::Null)
        };
        accessibility(
            name,
            &read(baseline, old),
            &read(current, new),
            old,
            new,
            &mut report,
        );
    }
    report.added = after
        .keys()
        .filter(|name| !before.contains_key(*name))
        .cloned()
        .collect();
    write(current, &report);
    Some(report)
}

/// Fraction of pixels that changed, writing a highlighted diff when any did.
fn pixels(before: &Path, after: &Path, diff: &Path) -> Option<f64> {
    let before = image::open(before).ok()?.to_rgba8();
    let after = image::open(after).ok()?.to_rgba8();
    if before.dimensions() != after.dimensions() {
        return None;
    }
    let mut changed = 0_u64;
    let mut highlight = RgbaImage::new(after.width(), after.height());
    for ((x, y, new), old) in after.enumerate_pixels().zip(before.pixels()) {
        let moved = new
            .0
            .iter()
            .zip(old.0)
            .any(|(a, b)| a.abs_diff(b) > TOLERANCE);
        let faded = |channel: u8| (u16::from(channel) / 4 + 160) as u8;
        highlight.put_pixel(
            x,
            y,
            if moved {
                changed += 1;
                Rgba([230, 0, 140, 255])
            } else {
                Rgba([faded(new[0]), faded(new[1]), faded(new[2]), 255])
            },
        );
    }
    if changed > 0 {
        super::record::save_image(&highlight, diff);
    }
    Some(changed as f64 / f64::from(after.width() * after.height()))
}

type Control = (String, String);

fn controls(snapshot: &Value) -> BTreeMap<Control, Vec<Value>> {
    let mut controls: BTreeMap<Control, Vec<Value>> = BTreeMap::new();
    for node in snapshot["nodes"].as_array().into_iter().flatten() {
        // Scroll containers follow content size; the controls inside them are what users name.
        let scrolling = matches!(node["role"].as_str(), Some("ScrollView" | "ScrollBar"));
        if node["interactive"].as_bool() == Some(true) && !scrolling {
            let key = (
                node["role"].as_str().unwrap_or("").to_owned(),
                node["label"].as_str().unwrap_or("").to_owned(),
            );
            controls.entry(key).or_default().push(node.clone());
        }
    }
    controls
}

fn size(node: &Value) -> Option<(f64, f64)> {
    let bounds = node["bounds"].as_array()?;
    let at = |index: usize| bounds.get(index).and_then(Value::as_f64);
    Some((at(2)? - at(0)?, at(3)? - at(1)?))
}

fn lint(frame: &Value) -> BTreeSet<String> {
    frame["lint"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        // Positions move with layout; the finding itself is what matters.
        .map(|finding| finding.split(" at [").next().unwrap_or(finding).to_owned())
        .collect()
}

fn accessibility(
    name: &str,
    before: &Value,
    after: &Value,
    old: &Value,
    new: &Value,
    report: &mut Report,
) {
    let (before, after) = (controls(before), controls(after));
    for ((role, label), nodes) in &before {
        let Some(current) = after.get(&(role.clone(), label.clone())) else {
            report
                .review
                .push(format!("{name}: {role} {label:?} is gone or renamed"));
            continue;
        };
        if current.len() < nodes.len() {
            report.review.push(format!(
                "{name}: {role} {label:?} appears {} times, was {}",
                current.len(),
                nodes.len()
            ));
        }
        for (old, new) in nodes.iter().zip(current) {
            if old["focusable"] == json!(true) && new["focusable"] != json!(true) {
                report
                    .regressions
                    .push(format!("{name}: {role} {label:?} is no longer focusable"));
            }
            if old["disabled"] != json!(true) && new["disabled"] == json!(true) {
                report
                    .review
                    .push(format!("{name}: {role} {label:?} is now disabled"));
            }
            if let (Some((ow, oh)), Some((nw, nh))) = (size(old), size(new))
                && (nw < ow.min(MINIMUM_TARGET) || nh < oh.min(MINIMUM_TARGET))
            {
                report.regressions.push(format!(
                    "{name}: {role} {label:?} target shrank from {ow}x{oh} to {nw}x{nh}"
                ));
            }
        }
    }
    let (was, now) = (lint(old), lint(new));
    for finding in now.difference(&was) {
        report
            .regressions
            .push(format!("{name}: new finding: {finding}"));
    }
}

fn write(root: &Path, report: &Report) {
    let mut text = String::from("# Writer gallery comparison\n\n");
    let _ = writeln!(
        text,
        "{} unchanged, {} changed, {} missing, {} new frames.\n",
        report.unchanged,
        report.changed.len(),
        report.missing.len(),
        report.added.len()
    );
    let section = |text: &mut String, title: &str, items: &[String]| {
        if items.is_empty() {
            return;
        }
        let _ = writeln!(text, "## {title}\n");
        for item in items {
            let _ = writeln!(text, "- {item}");
        }
        text.push('\n');
    };
    section(
        &mut text,
        "Accessibility regressions (never permitted)",
        &report.regressions,
    );
    section(
        &mut text,
        "Accessibility changes to justify",
        &report.review,
    );
    let changed: Vec<_> = report
        .changed
        .iter()
        .map(|(name, ratio)| format!("{name}: {:.2}% of pixels (diff/{name}.webp)", ratio * 100.))
        .collect();
    section(&mut text, "Visual changes", &changed);
    section(&mut text, "Missing frames", &report.missing);
    section(&mut text, "New frames", &report.added);
    let _ = fs::write(root.join("report.md"), text);
}
