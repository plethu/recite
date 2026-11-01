//! Prepared card text separates authored prose from structural annotations.
use recite_writer_model::{PassageKind, ScriptEntry};

pub struct Synopsis {
    pub speaker: String,
    pub prose: String,
    pub annotations: Vec<String>,
}
pub fn synopsis(entries: &[ScriptEntry]) -> Synopsis {
    fn scan(
        entries: &[ScriptEntry],
        effects: &mut usize,
        conditions: &mut usize,
        prose: &mut Option<(String, String)>,
    ) {
        for entry in entries {
            match entry {
                ScriptEntry::Passage(p) if prose.is_none() => {
                    let speaker = match &p.kind {
                        PassageKind::Dialogue { speaker } => speaker
                            .as_deref()
                            .map(crate::choices::display_name)
                            .unwrap_or_else(|| "Dialogue".into()),
                        PassageKind::Choice { .. } => "Reply".into(),
                    };
                    *prose = Some((speaker, p.text.clone()));
                }
                ScriptEntry::Effect(_) => *effects += 1,
                ScriptEntry::Group { entries, .. } => {
                    *conditions += 1;
                    scan(entries, effects, conditions, prose);
                }
                _ => {}
            }
        }
    }
    let (mut effects, mut conditions, mut first) = (0, 0, None);
    scan(entries, &mut effects, &mut conditions, &mut first);
    let (speaker, prose) = first.unwrap_or_else(|| ("".into(), "Continue".into()));
    let mut annotations = Vec::new();
    if conditions > 0 {
        annotations.push("◇ Conditional".into());
    }
    if effects > 0 {
        annotations.push(format!(
            "↗ {effects} effect{}",
            if effects == 1 { "" } else { "s" }
        ));
    }
    Synopsis {
        speaker,
        prose,
        annotations,
    }
}
