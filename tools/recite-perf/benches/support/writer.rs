use std::sync::Arc;

use recite_core::{DocumentKey, schema::load_schema_manifest_str};
use recite_writer_model::{Document, PassageKind, ProjectContext, ScriptBlock, ScriptEntry};

pub(super) struct ScriptWorkload {
    pub(super) document: Document,
    passages: usize,
}

pub(super) struct ProjectedScript {
    pub(super) workload: ScriptWorkload,
    pub(super) script: Arc<[ScriptBlock]>,
}

pub(super) fn effects(passages: usize) -> ScriptWorkload {
    let mut source =
        String::from("# Original deterministic effects/prose stress probe.\n:: relay default\n");
    for id in 0..passages {
        source.push_str(&format!(
            "! immediate change_disposition(mara, 1)\n> courier_{id}@{id:020x} speaker=mara\n  Café — the courier returns. Beacon{id} remains unanswered.\n\n"
        ));
    }
    source.push_str("-> END\n");
    let schema = load_schema_manifest_str(
        "writer_examples.json",
        include_str!("../../../../fixtures/schema/valid/writer_examples.json"),
    )
    .schema
    .unwrap_or_else(|| panic!("Writer fixture schema is invalid"));
    let key = DocumentKey::new("effects.recite")
        .unwrap_or_else(|error| panic!("Writer fixture key is invalid: {error}"));
    let document = Document::in_project(
        key,
        source,
        ProjectContext {
            documents: Vec::new(),
            schema: Some(schema),
        },
    )
    .unwrap_or_else(|error| panic!("Writer fixture failed to open: {error}"));
    assert!(document.diagnostics().is_empty());
    ScriptWorkload { document, passages }
}

pub(super) fn validate_script(projected: ProjectedScript) {
    let passages = projected.workload.passages;
    assert_eq!(projected.script.len(), 1);
    let block = &projected.script[0];
    assert_eq!(block.id, "relay");
    assert!(block.is_default);
    assert_eq!(block.entries.len(), passages * 2 + 1);
    for (id, pair) in block.entries[..passages * 2].chunks_exact(2).enumerate() {
        assert_eq!(
            pair[0],
            ScriptEntry::Effect("! immediate change_disposition(mara, 1)".into())
        );
        let ScriptEntry::Passage(passage) = &pair[1] else {
            panic!("effect {id} did not retain its following passage");
        };
        assert_eq!(passage.id, format!("{id:020x}"));
        assert_eq!(passage.label, format!("courier_{id}"));
        assert_eq!(passage.section, "relay");
        assert_eq!(
            passage.text,
            format!("Café — the courier returns. Beacon{id} remains unanswered.")
        );
        assert_eq!(
            passage.kind,
            PassageKind::Dialogue {
                speaker: Some("mara".into())
            }
        );
    }
    assert_eq!(block.entries[passages * 2], ScriptEntry::Jump("END".into()));
}
