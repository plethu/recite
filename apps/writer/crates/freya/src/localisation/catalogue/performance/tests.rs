use super::super::*;
use criterion::measurement::{Measurement, WallTime};

#[test]
#[ignore = "opt-in catalogue-scale timing; not native painting"]
fn catalogue_scale_timing() -> Result<(), Box<dyn std::error::Error>> {
    let entries = std::env::var("RECITE_BENCH_ENTRIES")
        .ok()
        .map(|n| n.parse::<usize>())
        .transpose()?
        .unwrap_or(10000);
    let edits = std::env::var("RECITE_BENCH_EDITS")
        .ok()
        .map(|n| n.parse::<usize>())
        .transpose()?
        .unwrap_or(20)
        .min(entries);
    let mut source = String::new();
    for i in 0..entries {
        source.push_str(&format!("#. file: scenes/chapter.recite\n#. block: chapter_start\n# translator note\nmsgctxt \"{i:020}\"\nmsgid \"Hello {{name}} — café dialogue passage {i}\"\nmsgstr \"Bonjour {{name}} — scène traduite {i}\"\n\n"));
    }
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("fr.po");
    std::fs::write(&path, &source)?;
    let mut catalogue = Catalogue::open(&path)?;
    let mut update_ms = Vec::new();
    for i in 0..edits {
        let draft = Draft {
            forms: vec![format!("Salut {{name}} — traduit {i}")],
            reviewed: i % 2 == 0,
        };
        let started = WallTime.start();
        catalogue.update(PoEntryId::new(i), draft);
        update_ms.push(WallTime.end(started).as_secs_f64() * 1000.);
    }
    let mut snapshot_ms = Vec::new();
    for _ in 0..20 {
        let started = WallTime.start();
        let snapshot = catalogue.recovery_snapshot();
        snapshot_ms.push(WallTime.end(started).as_secs_f64() * 1000.);
        std::hint::black_box(&snapshot);
    }
    let index_started = WallTime.start();
    let index = crate::localisation::queue::filter::QueueIndex::new(&catalogue, &[]);
    let index_ms = WallTime.end(index_started).as_secs_f64() * 1000.;
    let rebuild_started = WallTime.start();
    let replacement = crate::localisation::queue::filter::QueueIndex::new(&catalogue, &[]);
    let index_rebuild_ms = WallTime.end(rebuild_started).as_secs_f64() * 1000.;
    std::hint::black_box(&replacement);
    drop(replacement);
    let mut filtering = Vec::new();
    for query in ["", "bonjour", "chapter start", "absent", "traduit"] {
        let mut times = Vec::new();
        let mut count = 0;
        for _ in 0..5 {
            let started = WallTime.start();
            let matches = index.matching_entries(&catalogue, query, false);
            times.push(WallTime.end(started).as_secs_f64() * 1000.);
            count = matches.len();
            std::hint::black_box(&matches);
        }
        filtering.push(serde_json::json!({"query":query,"count":count,"ms":times}));
    }
    let started = WallTime.start();
    catalogue.save_all()?;
    let save_ms = WallTime.end(started).as_secs_f64() * 1000.;
    assert!(!catalogue.dirty());
    let reopened = Catalogue::open(&path)?;
    assert_eq!(reopened.document.entries().len(), entries);
    for i in 0..edits {
        let draft = reopened.draft(PoEntryId::new(i)).ok_or("saved draft")?;
        assert_eq!(draft.forms, [format!("Salut {{name}} — traduit {i}")]);
        assert_eq!(draft.reviewed, i % 2 == 0);
    }
    println!(
        "{}",
        serde_json::json!({"entries":entries,"edits":edits,"source_bytes":source.len(),"update_ms":update_ms,"snapshot_ms":snapshot_ms,"index_ms":index_ms,"index_rebuild_ms":index_rebuild_ms,"filtering":filtering,"save_ms":save_ms,"profile":if cfg!(debug_assertions) { "test" } else { "release-test" }})
    );
    Ok(())
}

#[test]
#[ignore = "opt-in authored catalogue workflow timing; excludes native painting"]
fn catalogue_authored_workflow_timing() -> Result<(), Box<dyn std::error::Error>> {
    let example = recite_writer_model::WRITER_EXAMPLES[0].open()?;
    let passages = example.document().passage_snapshot()?;
    let template = example
        .document()
        .extract_catalogue()
        .catalog
        .ok_or("authored template")?
        .to_pot_string();
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("fr.po");
    std::fs::write(&path, &template)?;
    let start = WallTime.start();
    let mut catalogue = Catalogue::open_recoverable(&path)?;
    let open_ms = WallTime.end(start).as_secs_f64() * 1000.;
    let start = WallTime.start();
    let index = crate::localisation::queue::filter::QueueIndex::new(&catalogue, &passages);
    let index_ms = WallTime.end(start).as_secs_f64() * 1000.;
    let drafts = catalogue
        .document
        .entries()
        .iter()
        .filter(|e| !e.is_header() && !e.is_obsolete())
        .map(|entry| (entry.id(), entry.source_text().to_owned()))
        .collect::<Vec<_>>();
    let mut update_ms = Vec::new();
    for (id, text) in &drafts {
        let draft = Draft {
            forms: vec![text.clone()],
            reviewed: true,
        };
        let start = WallTime.start();
        catalogue.update(*id, draft);
        update_ms.push(WallTime.end(start).as_secs_f64() * 1000.);
    }
    assert_eq!(
        index.matching_entries(&catalogue, "", true).len(),
        drafts.len()
    );
    let start = WallTime.start();
    catalogue.save_all()?;
    let save_ms = WallTime.end(start).as_secs_f64() * 1000.;
    drop(catalogue);
    let reopened = Catalogue::open_recoverable(&path)?;
    assert!(!reopened.dirty());
    for (id, text) in &drafts {
        let draft = reopened.draft(*id).ok_or("saved authored translation")?;
        assert_eq!(draft.forms, std::slice::from_ref(text));
        assert!(draft.reviewed);
    }
    println!(
        "{}",
        serde_json::json!({"fixture":"relay_hub","entries":drafts.len(),"source_bytes":template.len(),"open_ms":open_ms,"index_ms":index_ms,"update_ms":update_ms,"save_ms":save_ms,"durable_reopen_verified":true})
    );
    Ok(())
}
