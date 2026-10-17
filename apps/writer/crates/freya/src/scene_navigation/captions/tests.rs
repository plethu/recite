use super::*;
use recite_writer_model::{Document, ScriptEntry};

#[test]
fn prose_and_labels_reuse_captions_while_names_badges_and_order_invalidate()
-> Result<(), Box<dyn std::error::Error>> {
    let blocks = Document::new(
        ":: first default\n> line@11111111111111111111\n  Before.\n-> second\n:: second\n-> END\n",
    )?
    .script()?;
    let original = refresh(&blocks, None);
    assert_eq!(original[0].label, "Start · First");
    assert_eq!(original[1].label, "Second · end");
    let prose = Document::new(
        ":: first default\n> line@11111111111111111111\n  Changed.\n-> second\n:: second\n-> END\n",
    )?
    .script()?;
    assert!(Arc::ptr_eq(&original, &refresh(&prose, Some(&original))));
    let mut changed = blocks.clone();
    changed[1].entries = vec![ScriptEntry::Jump("first".into())];
    assert!(!Arc::ptr_eq(&original, &refresh(&changed, Some(&original))));
    changed = blocks.clone();
    changed[0].is_default = false;
    assert!(!Arc::ptr_eq(&original, &refresh(&changed, Some(&original))));
    changed = blocks.clone();
    changed[0].id = "ÉLAN_😀".into();
    let renamed = refresh(&changed, Some(&original));
    assert_eq!(renamed[0].folded, renamed[0].label.to_lowercase());
    assert!(!Arc::ptr_eq(&original, &renamed));
    changed = blocks;
    changed.reverse();
    assert!(!Arc::ptr_eq(&original, &refresh(&changed, Some(&original))));
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
        let blocks: Vec<_> = (0..count)
            .map(|i| ScriptBlock {
                id: format!("beat_{i}"),
                is_default: i == 0,
                entries: vec![ScriptEntry::Jump(if i + 1 == count {
                    "END".into()
                } else {
                    format!("beat_{}", i + 1)
                })],
            })
            .collect();
        let previous = refresh(&blocks, None);
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
                    "cold" => Snapshot::Current(refresh(&blocks, None)),
                    _ => Snapshot::Current(refresh(&blocks, Some(&previous))),
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
