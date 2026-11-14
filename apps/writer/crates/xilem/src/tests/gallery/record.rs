//! Frames, accessibility snapshots and the manifest that reviewers and comparisons read.
use super::stage::Frame;
use image::{RgbaImage, imageops};
use masonry::accesskit::{Action, Role};
use masonry::testing::TestHarness;
use serde_json::{Value, json};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use xilem::masonry::widgets::Passthrough;

/// WCAG 2.2 AA §2.5.8 minimum target, in logical pixels.
pub(crate) const MINIMUM_TARGET: f64 = 24.;

/// What one gallery entry shows, before any frame is taken.
pub(crate) struct Meta<'a> {
    /// Stable identifier: comparisons match baseline frames by it, so never reuse one.
    pub id: &'a str,
    pub group: &'a str,
    pub title: &'a str,
    /// What a reviewer should look at here: the state, data and interaction captured.
    pub intent: &'a str,
}

struct Entry {
    id: String,
    group: String,
    title: String,
    intent: String,
    variant: String,
    frames: Vec<Value>,
}

pub(crate) struct Recorder {
    root: PathBuf,
    only: Vec<String>,
    limit: Option<&'static [&'static str]>,
    variant: String,
    current: Option<Entry>,
    entries: Vec<Value>,
}
impl Recorder {
    pub fn new(root: &Path, only: Vec<String>) -> std::io::Result<Self> {
        for directory in ["frames", "a11y", "strips"] {
            let path = root.join(directory);
            if path.exists() {
                fs::remove_dir_all(&path)?;
            }
            fs::create_dir_all(path)?;
        }
        Ok(Self {
            root: root.to_owned(),
            only,
            limit: None,
            variant: String::new(),
            current: None,
            entries: vec![],
        })
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn set_variant(&mut self, variant: &str) {
        variant.clone_into(&mut self.variant);
    }
    /// Record only these scene ids until the limit changes; `None` records every scene.
    pub fn limit(&mut self, scenes: Option<&'static [&'static str]>) {
        self.limit = scenes;
    }
    fn wanted(&self, scene: &str, id: &str) -> bool {
        self.limit.is_none_or(|limit| limit.contains(&scene))
            && (self.only.is_empty() || self.only.iter().any(|only| id.contains(only.as_str())))
    }

    /// Record one entry; `body` builds its stage and takes one frame or a sequence of frames.
    pub fn scene(&mut self, meta: Meta<'_>, body: impl FnOnce(&mut Self)) {
        let id = format!("{}--{}", meta.id, self.variant);
        if !self.wanted(meta.id, &id) {
            return;
        }
        eprintln!("gallery: {id}");
        self.current = Some(Entry {
            id,
            group: meta.group.into(),
            title: meta.title.into(),
            intent: meta.intent.into(),
            variant: self.variant.clone(),
            frames: vec![],
        });
        body(self);
        let Some(entry) = self.current.take() else {
            return;
        };
        if entry.frames.len() > 1 {
            self.strip(&entry);
        }
        self.entries.push(json!({
            "id": entry.id,
            "group": entry.group,
            "title": entry.title,
            "intent": entry.intent,
            "variant": entry.variant,
            "kind": if entry.frames.len() > 1 { "sequence" } else { "still" },
            "strip": (entry.frames.len() > 1).then(|| format!("strips/{}.png", entry.id)),
            "frames": entry.frames,
        }));
    }

    pub fn frame(
        &mut self,
        caption: &str,
        image: RgbaImage,
        harness: &TestHarness<Passthrough>,
        frame: Frame,
        warnings: Vec<String>,
    ) {
        let Some(entry) = self.current.as_mut() else {
            return;
        };
        let name = format!("{}--{:02}", entry.id, entry.frames.len());
        let file = format!("frames/{name}.png");
        save_png(&image, &self.root.join(&file));
        let (nodes, lint) = accessibility(harness, frame.scale);
        let a11y = format!("a11y/{name}.json");
        // One node per line keeps snapshots small and their diffs readable.
        let lines: Vec<String> = nodes.iter().map(Value::to_string).collect();
        let snapshot = format!(
            "{{\"frame\":{},\"nodes\":[\n{}\n]}}\n",
            Value::from(name.as_str()),
            lines.join(",\n")
        );
        fs::write(self.root.join(&a11y), snapshot).expect("write accessibility snapshot");
        entry.frames.push(json!({
            "name": name,
            "caption": caption,
            "file": file,
            "a11y": a11y,
            "size": frame.label(),
            "scale": frame.scale,
            "warnings": warnings,
            "lint": lint,
        }));
    }

    fn strip(&self, entry: &Entry) {
        let frames: Vec<RgbaImage> = entry
            .frames
            .iter()
            .filter_map(|frame| frame["file"].as_str())
            .filter_map(|file| image::open(self.root.join(file)).ok())
            .map(|image| {
                let image = image.to_rgba8();
                let width = image.width() / 2;
                let height = image.height() / 2;
                imageops::resize(&image, width, height, imageops::FilterType::Triangle)
            })
            .collect();
        let gap = 8;
        let width = frames.iter().map(|frame| frame.width() + gap).sum::<u32>() + gap;
        let height = frames.iter().map(RgbaImage::height).max().unwrap_or(1) + gap * 2;
        let mut strip = RgbaImage::from_pixel(width, height, image::Rgba([40, 36, 49, 255]));
        let mut x = gap;
        for frame in &frames {
            imageops::overlay(&mut strip, frame, i64::from(x), i64::from(gap));
            x += frame.width() + gap;
        }
        save_png(&strip, &self.root.join(format!("strips/{}.png", entry.id)));
    }

    /// Write the manifest and contact sheet; returns the manifest for comparison.
    pub fn finish(self, settings: Value) -> Value {
        let manifest = json!({
            "format": 1,
            "settings": settings,
            "entries": self.entries,
        });
        fs::write(
            self.root.join("manifest.json"),
            serde_json::to_string_pretty(&manifest).unwrap_or_default() + "\n",
        )
        .expect("write gallery manifest");
        fs::write(self.root.join("index.html"), contact_sheet(&manifest))
            .expect("write contact sheet");
        manifest
    }
}

fn save_png(image: &RgbaImage, path: &Path) {
    let mut bytes = std::io::Cursor::new(vec![]);
    image
        .write_to(&mut bytes, image::ImageFormat::Png)
        .expect("encode frame");
    let bytes = bytes.into_inner();
    // Committed baselines stay small; fall back to the plain encoding if optimisation fails.
    let bytes =
        oxipng::optimize_from_memory(&bytes, &oxipng::Options::from_preset(2)).unwrap_or(bytes);
    fs::write(path, bytes).expect("write frame");
}

/// The accessibility tree in reading order, plus automatic findings for interactive nodes.
fn accessibility(harness: &TestHarness<Passthrough>, scale: f64) -> (Vec<Value>, Vec<String>) {
    let tree = harness.access_tree();
    let state = tree.state();
    let mut nodes = vec![];
    let mut lint = vec![];
    let mut pending = vec![state.root()];
    while let Some(node) = pending.pop() {
        if node.is_hidden() {
            continue;
        }
        let mut children: Vec<_> = node.children().collect();
        children.reverse();
        pending.extend(children);
        let data = node.data();
        let role = node.role();
        // A text run repeats its label's text; the label already carries it.
        if role == Role::TextRun {
            continue;
        }
        let interactive = data.supports_action(Action::Click)
            || data.supports_action(Action::Focus)
            || node.is_text_input();
        let label = node.label().filter(|label| !label.trim().is_empty());
        let value = node.value();
        if !interactive && label.is_none() && value.is_none() {
            continue;
        }
        let bounds = node
            .bounding_box()
            .map(|rect| [rect.x0, rect.y0, rect.x1, rect.y1].map(|edge| (edge / scale).round()));
        // Scroll containers and their bars are reached by wheel and keys over their content.
        let scrolling = matches!(role, Role::ScrollView | Role::ScrollBar);
        let role = format!("{role:?}");
        if interactive && !node.is_disabled() && !scrolling {
            let name = label.as_deref().unwrap_or("");
            if label.is_none() && value.as_deref().is_none_or(str::is_empty) {
                lint.push(format!("unnamed {role} at {bounds:?}"));
            }
            // A text field's node covers only its text line; its padded frame takes the click.
            if let Some([x0, y0, x1, y1]) = bounds
                && !node.is_text_input()
                && x1 > x0
                && y1 > y0
                && (x1 - x0 < MINIMUM_TARGET || y1 - y0 < MINIMUM_TARGET)
            {
                lint.push(format!(
                    "{role} {name:?} target {}x{} is under {MINIMUM_TARGET} px",
                    x1 - x0,
                    y1 - y0
                ));
            }
        }
        nodes.push(json!({
            "role": role,
            "label": label,
            "value": value,
            "interactive": interactive,
            "focusable": data.supports_action(Action::Focus),
            "focused": node.is_focused(),
            "disabled": node.is_disabled(),
            "bounds": bounds,
        }));
    }
    (nodes, lint)
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn contact_sheet(manifest: &Value) -> String {
    let mut html = String::from(concat!(
        "<!doctype html><meta charset=utf-8><meta name=viewport content='width=device-width'>",
        "<title>Writer gallery</title><style>",
        ":root{color-scheme:light dark;--ink:#302431;--paper:#f3e9df;--rule:#c9b8a8}",
        "@media (prefers-color-scheme:dark){:root{--ink:#f3e9df;--paper:#1d1720;--rule:#4a3d48}}",
        "body{margin:0;padding:16px;background:var(--paper);color:var(--ink);",
        "font:15px/1.5 system-ui,sans-serif}h2{margin:2em 0 .5em;border-bottom:1px solid var(--rule)}",
        "section{margin:0 0 2em}figure{display:inline-block;margin:0 12px 12px 0;vertical-align:top;",
        "max-width:min(100%,620px)}img{max-width:100%;border:1px solid var(--rule)}",
        "figcaption{font-size:13px}.warn{color:#a3342f}details{font-size:13px}",
        "</style><h1>Writer gallery</h1>"
    ));
    let entries = manifest["entries"].as_array().cloned().unwrap_or_default();
    let mut group = "";
    for entry in &entries {
        let this = entry["group"].as_str().unwrap_or("");
        if this != group {
            let _ = write!(html, "<h2>{}</h2>", escape(this));
            group = this;
        }
        let _ = write!(
            html,
            "<section id='{id}'><h3>{title} <small>{variant}</small></h3><p>{intent}</p>",
            id = escape(entry["id"].as_str().unwrap_or("")),
            title = escape(entry["title"].as_str().unwrap_or("")),
            variant = escape(entry["variant"].as_str().unwrap_or("")),
            intent = escape(entry["intent"].as_str().unwrap_or("")),
        );
        for frame in entry["frames"].as_array().into_iter().flatten() {
            let file = escape(frame["file"].as_str().unwrap_or(""));
            let caption = escape(frame["caption"].as_str().unwrap_or(""));
            let _ = write!(
                html,
                "<figure><a href='{file}'><img src='{file}' alt='{caption}' loading=lazy></a>\
                 <figcaption>{caption} · <a href='{a11y}'>accessibility tree</a></figcaption>",
                a11y = escape(frame["a11y"].as_str().unwrap_or("")),
            );
            for (class, key) in [("warn", "warnings"), ("", "lint")] {
                let notes: Vec<_> = frame[key].as_array().into_iter().flatten().collect();
                if !notes.is_empty() {
                    let _ = write!(
                        html,
                        "<details class='{class}'><summary>{} {key}</summary><ul>",
                        notes.len()
                    );
                    for note in notes {
                        let _ = write!(html, "<li>{}", escape(note.as_str().unwrap_or("")));
                    }
                    html.push_str("</ul></details>");
                }
            }
            html.push_str("</figure>");
        }
        html.push_str("</section>");
    }
    html
}
