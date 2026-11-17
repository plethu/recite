use super::compare::compare;
use super::record::Recorder;
use super::*;
use serde_json::json;
use std::fs;

/// Scenes whose layout is worth checking at the window, scale and contrast extremes.
const EXTREMES: &[&str] = &[
    "start.welcome",
    "script.sample",
    "script.empty-beat",
    "script.galley",
    "script.draft",
    "script.small-type",
    "map.sample",
    "map.split",
    "localise.beat",
    "overlay.settings",
    "overlay.commands",
    "overlay.close",
    "preview.configure",
    "script.disk-changed",
    "script.build-result",
    "script.rename",
    "script.choices",
    "map.rename",
    "start.create",
    "source.repair",
    "localise.queue",
    "localise.none",
    "tools.build",
    "tools.rename",
    "tools.declarations",
    "tools.disk",
    "tools.updates",
];

/// Transitions whose meaning depends on contrast alone.
const MONO_TRANSITIONS: &[&str] = &["flow.tab-walk"];

#[test]
#[ignore = "renders the design gallery; run it with `just writer gallery`"]
fn capture_gallery() {
    let settings = Settings::from_env();
    let mut record =
        Recorder::new(&settings.output, settings.only.clone()).expect("gallery output");
    for variant in Variant::ALL {
        let variant = Variant {
            frame: variant.frame.at(variant.frame.scale * settings.scale),
            ..variant
        };
        record.set_variant(variant.id);
        let full = matches!(variant.id, "light" | "dark");
        record.limit((!full).then_some(EXTREMES));
        scenes::stills(&mut record, variant);
        scenes::in_place(&mut record, variant);
        if full {
            scenes::transitions(&mut record, variant, &settings);
        } else if variant.id == "mono" {
            // Focus rings must hold without hue, so the Tab walk also runs in monochrome.
            record.limit(Some(MONO_TRANSITIONS));
            scenes::transitions(&mut record, variant, &settings);
        }
        // Control states, filled focus rings included, must hold in monochrome too.
        if full || variant.id == "mono" {
            record.limit(None);
            scenes::specimen(&mut record, variant);
        }
    }
    let root = record.root().to_owned();
    let manifest = record.finish(json!({
        "scale": settings.scale,
        "motion_ms": settings.motion,
        "only": settings.only,
    }));
    eprintln!("gallery: wrote {}", root.join("index.html").display());
    let Some(baseline) = &settings.baseline else {
        eprintln!("gallery: no baseline to compare");
        return;
    };
    let report = compare(&root, &manifest, baseline).expect("readable baseline");
    eprintln!(
        "gallery: {} unchanged, {} changed, {} accessibility regressions, {} to justify; see {}",
        report.unchanged,
        report.changed.len(),
        report.regressions.len(),
        report.review.len(),
        root.join("report.md").display()
    );
    assert!(
        !settings.strict || report.regressions.is_empty(),
        "accessibility regressions against the baseline:\n{}",
        report.regressions.join("\n")
    );
}

fn write_capture(root: &std::path::Path, nodes: serde_json::Value, lint: serde_json::Value) {
    fs::create_dir_all(root.join("a11y")).unwrap();
    fs::write(
        root.join("a11y/scene--light--00.json"),
        json!({ "nodes": nodes }).to_string(),
    )
    .unwrap();
    fs::write(
        root.join("manifest.json"),
        json!({ "entries": [{ "id": "scene--light", "variant": "light", "frames": [{
            "name": "scene--light--00",
            "file": "frames/scene--light--00.webp",
            "a11y": "a11y/scene--light--00.json",
            "lint": lint,
        }]}]})
        .to_string(),
    )
    .unwrap();
}

#[test]
fn comparison_blocks_lost_focus_shrunk_targets_and_new_findings_but_lists_renames_for_review() {
    let sized = |role: &str, label: &str, focusable: bool, (width, height): (f64, f64)| {
        json!({ "role": role, "label": label, "interactive": true, "focusable": focusable,
                "disabled": false, "bounds": [0., 0., width, height] })
    };
    let control =
        |label: &str, focusable: bool, width: f64| sized("Button", label, focusable, (width, 32.));
    let focused = |mut node: serde_json::Value| {
        node["focused"] = json!(true);
        node
    };
    let baseline = tempfile::tempdir().unwrap();
    write_capture(
        baseline.path(),
        json!([
            control("Save", true, 64.),
            control("Map", true, 48.),
            focused(control("Close", true, 32.)),
            control("Pin", true, 110.),
            sized("Button", "Open", true, (120., 40.)),
            sized("MultilineTextInput", "Line", true, (400., 60.))
        ]),
        json!([]),
    );
    let current = tempfile::tempdir().unwrap();
    write_capture(
        current.path(),
        json!([
            control("Save", false, 64.),
            focused(control("Map", true, 18.)),
            control("Exit", true, 32.),
            // An icon face that replaced the text: 30 high where the reference was 32.
            sized("Button", "Pin", true, (30., 30.)),
            // Tighter than the reference but still 32 high: a larger target may come down to 32.
            sized("Button", "Open", true, (32., 32.)),
            // One line shorter: a multiline field's height follows its text.
            sized("MultilineTextInput", "Line", true, (400., 40.))
        ]),
        json!(["unnamed Button at [0.0, 0.0, 10.0, 10.0]"]),
    );
    let manifest: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(current.path().join("manifest.json")).unwrap())
            .unwrap();
    let report = compare(current.path(), &manifest, baseline.path()).unwrap();
    assert_eq!(report.regressions.len(), 4, "{:?}", report.regressions);
    assert!(
        report
            .regressions
            .iter()
            .any(|r| r.contains("\"Save\" is no longer focusable"))
    );
    assert!(
        report
            .regressions
            .iter()
            .any(|r| r.contains("\"Map\" target shrank"))
    );
    assert!(
        report
            .regressions
            .iter()
            .any(|r| r.contains("\"Pin\" target shrank from 110x32 to 30x30"))
    );
    assert!(
        report
            .regressions
            .iter()
            .any(|r| r.contains("new finding: unnamed Button"))
    );
    assert_eq!(
        report.review,
        [
            "scene--light--00: keyboard focus is on Button \"Map\" instead of Button \"Close\"",
            "scene--light--00: Button \"Close\" is gone or renamed",
        ]
    );
    assert!(current.path().join("report.md").exists());
}

#[test]
fn comparison_blocks_text_that_shrinks_and_lists_line_boxes_that_do_for_review() {
    let node = |role: &str, words: &str, height: f64, text: serde_json::Value| {
        let interactive = role != "Label";
        json!({ "role": role, "label": interactive.then_some(words),
                "value": (!interactive).then_some(words), "interactive": interactive,
                "focusable": interactive, "disabled": false,
                "bounds": [0., 0., 200., height], "text": text })
    };
    let baseline = tempfile::tempdir().unwrap();
    write_capture(
        baseline.path(),
        json!([
            // Recorded font sizes.
            node("Button", "Go to Watch", 48., json!([28.8, 36.])),
            node("Button", "Rename", 48., json!([14., 20.])),
            // A baseline from before font sizes were recorded: its line boxes stand in.
            node("Label", "Relay Desk", 71., json!(null)),
            node("Label", "Two lines of guidance", 40., json!(null)),
        ]),
        json!([]),
    );
    let current = tempfile::tempdir().unwrap();
    write_capture(
        current.path(),
        json!([
            node("Button", "Go to Watch", 48., json!([14.4, 21.])),
            // Within rounding.
            node("Button", "Rename", 48., json!([13.6, 20.])),
            node("Label", "Relay Desk", 29., json!([24., 29.])),
            // The same text in one line where it took two: no shrink.
            node("Label", "Two lines of guidance", 20., json!([14., 20.])),
        ]),
        json!([]),
    );
    let manifest: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(current.path().join("manifest.json")).unwrap())
            .unwrap();
    let report = compare(current.path(), &manifest, baseline.path()).unwrap();
    assert_eq!(
        report.regressions,
        ["scene--light--00: Button \"Go to Watch\" text shrank from 28.8px to 14.4px"]
    );
    // A line box alone cannot tell type from leading: a reviewer justifies it.
    assert_eq!(
        report.review,
        ["scene--light--00: Label \"Relay Desk\" text shrank from a 35.5px line to 29px"]
    );
}

#[test]
fn variant_ids_are_unique_and_extremes_are_core_scenes() {
    let mut ids: Vec<_> = Variant::ALL.iter().map(|variant| variant.id).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), Variant::ALL.len());
    assert!(EXTREMES.iter().all(|id| id.contains('.')));
}

#[test]
fn saved_frames_decode_to_identical_pixels() {
    let mut frame = image::RgbaImage::new(97, 61);
    for (x, y, pixel) in frame.enumerate_pixels_mut() {
        // Grain-like noise and flat ink areas, the two kinds of content frames hold.
        let noise = (x * 7919 + y * 104_729) % 251;
        *pixel = image::Rgba([
            noise as u8,
            (x * 3) as u8,
            if y < 30 { 0x68 } else { 0xf3 },
            255,
        ]);
    }
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("frame.webp");
    super::record::save_image(&frame, &path);
    assert_eq!(image::open(&path).unwrap().to_rgba8(), frame);
}

#[test]
fn an_open_writer_takes_the_variant_like_a_project_opened_with_those_preferences() {
    for variant in Variant::ALL {
        let app = crate::app::App::with_writer(scenes::examples("relay_hub.recite"));
        let stage = scenes::stage(app, variant);
        let writer = stage.state.writer.as_ref().unwrap();
        let mut config = recite_config::WriterConfig::default();
        variant.configure(&mut config);
        assert_eq!(
            writer.palette,
            crate::presentation::palette(&config),
            "{}",
            variant.id
        );
        assert_eq!(
            writer.ui_scale(),
            crate::presentation::scale(&config),
            "{}",
            variant.id
        );
    }
}
