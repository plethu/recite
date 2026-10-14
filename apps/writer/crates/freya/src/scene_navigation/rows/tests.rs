use super::*;
use freya::prelude::EventHandler;
use recite_writer_model::{ScriptBlock, ScriptEntry};
use std::{cell::Cell, rc::Rc};

fn scene(name: &str, active: bool) -> SceneBranch {
    SceneBranch {
        caption: name.into(),
        active,
        open: EventHandler::new(|()| {}),
    }
}
fn captions(count: usize) -> Arc<[Caption]> {
    let blocks = (0..count)
        .map(|i| ScriptBlock {
            id: format!("beat_{i}"),
            is_default: i == 0,
            entries: vec![ScriptEntry::Jump("END".into())],
        })
        .collect::<Vec<_>>();
    super::super::captions::refresh(&blocks, None)
}

#[test]
fn selection_refresh_reuses_rows_and_new_callbacks_resolve_against_current_scenes() {
    let captions = captions(10_000);
    let mut cache = Cache::default();
    let initial = cache.refresh(
        &[scene("Active", true), scene("Other", false)],
        &captions,
        "",
        None,
    );
    assert_eq!(initial.items.len(), 10_002);
    let calls = Rc::new(Cell::new(0));
    let changed = calls.clone();
    let scenes = [
        scene("Active", true),
        SceneBranch {
            caption: "Other".into(),
            active: false,
            open: EventHandler::new(move |()| changed.set(changed.get() + 1)),
        },
    ];
    for _ in 0..100 {
        let next = cache.refresh(&scenes, &captions, "", None);
        assert!(Arc::ptr_eq(&initial.items, &next.items));
        assert!(next.matches.is_none());
        let Row::Scene {
            scene_index,
            expanded: false,
        } = next.items[10_001]
        else {
            panic!("inactive scene row");
        };
        scenes[scene_index].open.call(());
    }
    assert_eq!(calls.get(), 100);
}

#[test]
fn replacing_or_dropping_the_single_cache_releases_old_captions() {
    let mut cache = Cache::default();
    for count in [1_000, 10_000] {
        let captions = captions(count);
        let weak = Arc::downgrade(&captions);
        cache.refresh(&[scene("Active", true)], &captions, "", None);
        drop(captions);
        assert!(weak.upgrade().is_some());
        cache.refresh(&[], &Arc::from([]), "", None);
        assert!(weak.upgrade().is_none());
    }
    let captions = captions(1_000);
    let weak = Arc::downgrade(&captions);
    cache.refresh(&[scene("Active", true)], &captions, "", None);
    drop(captions);
    drop(cache);
    assert!(weak.upgrade().is_none());
}

#[test]
fn filtering_expansion_and_caption_changes_rebuild_the_index_and_matches() {
    let captions = captions(3);
    let scenes = [scene("Élan", true), scene("Other", false)];
    let mut cache = Cache::default();
    let initial = cache.refresh(&scenes, &captions, "", None);
    assert_eq!(initial.items.len(), 5);
    let collapsed = cache.refresh(&scenes, &captions, "", Some("Élan"));
    assert_eq!(collapsed.items.len(), 2);
    assert!(!Arc::ptr_eq(&initial.items, &collapsed.items));
    let filtered = cache.refresh(&scenes, &captions, "BEAT 1", Some("Élan"));
    assert_eq!(
        &*filtered.items,
        &[
            Row::Scene {
                scene_index: 0,
                expanded: true
            },
            Row::Beat { caption_index: 1 }
        ]
    );
    assert_eq!(filtered.matches, Some(vec!["beat_1".into()]));
    let scene_match = cache.refresh(&scenes, &captions, "ÉLAN", Some("Élan"));
    assert_eq!(scene_match.items.len(), 4);
    assert_eq!(scene_match.matches.as_ref().map(Vec::len), Some(3));
    let inactive = cache.refresh(&scenes, &captions, "other", None);
    assert_eq!(
        &*inactive.items,
        &[Row::Scene {
            scene_index: 1,
            expanded: false
        }]
    );
    assert_eq!(inactive.matches, Some(vec![]));
    let missing = cache.refresh(&scenes, &captions, "absent", None);
    assert!(missing.items.is_empty());
    assert_eq!(missing.matches, Some(vec![]));
    let mut renamed = captions.to_vec();
    renamed[1].id = "renamed".into();
    let renamed = renamed.into();
    let next = cache.refresh(&scenes, &renamed, "ÉLAN", None);
    assert_eq!(
        next.matches,
        Some(vec!["beat_0".into(), "renamed".into(), "beat_2".into()])
    );
    let switched = cache.refresh(
        &[scene("Other", false), scene("Élan", true)],
        &captions,
        "",
        None,
    );
    assert_eq!(
        switched.items[0],
        Row::Scene {
            scene_index: 0,
            expanded: false
        }
    );
    assert_eq!(
        switched.items[1],
        Row::Scene {
            scene_index: 1,
            expanded: true
        }
    );
    let empty = cache.refresh(&[], &Arc::from([]), "", None);
    assert!(empty.items.is_empty());
    assert_eq!(empty.matches, Some(vec![]));
}

#[test]
#[ignore = "timing investigation, no latency threshold"]
fn navigation_row_refresh_timing() {
    use criterion::measurement::{Measurement, WallTime};
    for count in [1_000, 10_000] {
        let captions = captions(count);
        let scenes = [scene("Active", true), scene("Other", false)];
        let mut cache = Cache::default();
        let original = cache.refresh(&scenes, &captions, "", None).items;
        for operation in ["rebuild", "retained"] {
            let mut elapsed = Vec::new();
            for _ in 0..100 {
                let start = WallTime.start();
                let items = if operation == "rebuild" {
                    build(&scenes, &captions, "", None)
                } else {
                    cache.refresh(&scenes, &captions, "", None).items
                };
                elapsed.push(WallTime.end(start).as_secs_f64() * 1000.);
                assert_eq!(items, original);
                std::hint::black_box(items);
            }
            elapsed.sort_by(f64::total_cmp);
            println!(
                "{{\"beats\":{count},\"operation\":\"{operation}\",\"median_ms\":{}}}",
                elapsed[elapsed.len() / 2]
            );
        }
    }
}
