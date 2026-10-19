use super::*;
use recite_writer_model::Document;

#[test]
fn prose_and_labels_reuse_captions_while_names_badges_and_order_invalidate()
-> Result<(), Box<dyn std::error::Error>> {
    let source =
        ":: first default\n> line@11111111111111111111\n  Before.\n-> second\n:: second\n-> END\n";
    let document = Document::new(source)?;
    let original = refresh(document.block_summaries(), None);
    assert_eq!(original[0].label, "Start · First");
    assert_eq!(original[1].label, "Second · end");
    let prose = Document::new(source.replace("Before.", "Changed."))?;
    assert!(Arc::ptr_eq(
        &original,
        &refresh(prose.block_summaries(), Some(&original))
    ));
    for changed in [
        source.replace("-> END", "-> first"),
        source.replace(" default", ""),
        source.replace("first", "ÉLAN_😀"),
        ":: second\n-> END\n:: first default\n> line@11111111111111111111\n  Before.\n-> second\n"
            .into(),
    ] {
        let document = Document::new(changed)?;
        let refreshed = refresh(document.block_summaries(), Some(&original));
        assert!(!Arc::ptr_eq(&original, &refreshed));
        assert!(
            refreshed
                .iter()
                .all(|caption| caption.folded == caption.label.to_lowercase())
        );
    }
    assert!(refresh(&[], Some(&original)).is_empty());
    Ok(())
}

#[test]
#[ignore = "timing investigation, no latency threshold"]
fn caption_refresh_timing() {
    use criterion::measurement::{Measurement, WallTime};
    use std::collections::BTreeSet;
    enum Snapshot {
        Previous(Vec<(String, String)>),
        Current(Arc<[Caption]>),
    }
    for count in [1_000, 10_000] {
        let source = (0..count)
            .map(|i| {
                format!(
                    ":: beat_{i}{}\n-> {}\n",
                    if i == 0 { " default" } else { "" },
                    if i + 1 == count {
                        "END".into()
                    } else {
                        format!("beat_{}", i + 1)
                    }
                )
            })
            .collect::<String>();
        let document = Document::new(source).expect("generated document");
        let blocks = document.script().expect("valid Script");
        let summaries = document.block_summaries();
        let previous = refresh(summaries, None);
        for operation in ["previous", "cold", "warm"] {
            let mut elapsed = Vec::new();
            for _ in 0..5 {
                let start = WallTime.start();
                let result = match operation {
                    "previous" => {
                        let links = recite_writer_model::scene_links(&blocks);
                        let endings: BTreeSet<_> = links
                            .iter()
                            .filter(|l| l.destination == "END")
                            .map(|l| l.origin.as_str())
                            .collect();
                        Snapshot::Previous(
                            blocks
                                .iter()
                                .map(|block| {
                                    (
                                        block.id.clone(),
                                        format!(
                                            "{}{}{}",
                                            if block.is_default { "Start · " } else { "" },
                                            crate::palette::display_name(&block.id),
                                            if endings.contains(block.id.as_str()) {
                                                " · end"
                                            } else {
                                                ""
                                            }
                                        ),
                                    )
                                })
                                .collect::<Vec<_>>(),
                        )
                    }
                    "cold" => Snapshot::Current(refresh(summaries, None)),
                    _ => Snapshot::Current(refresh(summaries, Some(&previous))),
                };
                elapsed.push(WallTime.end(start).as_secs_f64() * 1_000.);
                let actual = match &result {
                    Snapshot::Previous(rows) => rows
                        .iter()
                        .map(|(id, label)| (id.as_str(), label.as_str()))
                        .collect::<Vec<_>>(),
                    Snapshot::Current(rows) => rows
                        .iter()
                        .map(|c| (c.id.as_str(), c.label.as_str()))
                        .collect(),
                };
                assert_eq!(
                    actual,
                    previous
                        .iter()
                        .map(|c| (c.id.as_str(), c.label.as_str()))
                        .collect::<Vec<_>>()
                );
            }
            elapsed.sort_by(f64::total_cmp);
            eprintln!(
                "caption refresh | debug host | beats={count} | {operation} | median={:.3} ms | excludes painting",
                elapsed[2]
            );
        }
    }
}

#[test]
fn summaries_preserve_script_badges_and_recoverable_outline()
-> Result<(), Box<dyn std::error::Error>> {
    for source in [
        ":: same default\n-> END\n:: same\n-> other\n:: other\n-> END\n",
        include_str!("../../../../../../../fixtures/recite/valid/core_language_spike.recite"),
        ":: start default\n:if ready()\n  -> END\n:else\n  -> next\n:: next\n:match mood()\n  :case _\n    -> END\n",
    ] {
        let document = Document::new(source)?;
        let script = document.script()?;
        let endings = recite_writer_model::scene_endings(&script);
        let captions = refresh(document.block_summaries(), None);
        for (caption, block) in captions.iter().zip(&script) {
            assert_eq!(caption.id, block.id);
            assert_eq!(caption.is_default, block.is_default);
            assert_eq!(caption.ending, endings.contains(block.id.as_str()));
        }
        assert_eq!(captions.len(), script.len());
    }
    for source in [
        ":: repair\n> line\n  Unfrozen.\n-> END\n",
        ":: repair\n-> END\n::\n",
    ] {
        let document = Document::new(source)?;
        assert!(document.script().is_err());
        let captions = refresh(document.block_summaries(), None);
        assert_eq!(captions[0].id, "repair");
        assert!(captions[0].ending);
    }
    Ok(())
}

#[test]
fn draft_notifications_keep_summary_identity_but_acceptance_and_replacement_refresh()
-> Result<(), Box<dyn std::error::Error>> {
    use recite_writer_model::{View, Workbench};
    let source = ":: first default\n-> END\n";
    let mut model = Workbench::from_document(Document::new(source)?, Some(View::Source))?;
    let initial = SummarySnapshot(model.document().summary_snapshot());
    let original = refresh(model.document().block_summaries(), None);
    for text in ["unfinished", ":: first\n-> other\n"] {
        model.set_draft(text.into());
        assert!(initial == SummarySnapshot(model.document().summary_snapshot()));
    }
    model.apply()?;
    let accepted = SummarySnapshot(model.document().summary_snapshot());
    assert!(initial != accepted);
    assert_eq!(
        refresh(model.document().block_summaries(), Some(&original))[0].label,
        "First"
    );
    let replacement = Document::new(source)?;
    assert_eq!(replacement.revision(), 1);
    let replaced = SummarySnapshot(replacement.summary_snapshot());
    assert_eq!(
        initial.0, replaced.0,
        "equal values can belong to a replacement document"
    );
    assert!(
        initial != replaced,
        "replacement identity must invalidate the memo"
    );
    assert_eq!(original[0].label, "Start · First · end");
    assert!(SummarySnapshot(None) == SummarySnapshot(None));
    assert!(initial != SummarySnapshot(None));
    Ok(())
}
