use super::*;
use freya_testing::prelude::*;
use recite_writer_model::{Document, ScriptEntry};
use std::{cell::Cell, sync::Arc};

thread_local! {
    pub(super) static ENTRY_VISITS: Cell<usize> = const { Cell::new(0) };
    static ENTRIES: Cell<usize> = const { Cell::new(0) };
}

fn app() -> impl IntoElement {
    use_init_theme(|| crate::palette::theme(false));
    let mut tick = use_state(|| 0usize);
    let mut blocks = use_state(|| {
        Arc::<[ScriptBlock]>::from([ScriptBlock {
            id: "start".into(),
            is_default: true,
            entries: vec![ScriptEntry::Source("padding".into()); ENTRIES.get()],
        }])
    });
    rect()
        .width(Size::fill())
        .child(label().text(format!("Revision {}", *tick.read())))
        .child(
            rect()
                .on_press(move |_| {
                    let next = *tick.peek() + 1;
                    tick.set(next);
                })
                .child(label().text("Redraw")),
        )
        .child(
            rect()
                .on_press(move |_| {
                    let source =
                        ":: start default\n? reply@11111111111111111111\n  Continue.\n  -> END\n";
                    let document = Document::new(source).unwrap();
                    let projected = document.script_snapshot().unwrap();
                    let reply = projected[0]
                        .entries
                        .iter()
                        .find(|entry| matches!(entry, ScriptEntry::Passage(_)))
                        .unwrap()
                        .clone();
                    let mut next = blocks.peek().to_vec();
                    next[0].entries.push(reply);
                    blocks.set(next.into());
                })
                .child(label().text("Replace snapshot")),
        )
        .child(Slug {
            key: "scene.recite".into(),
            blocks: Arc::downgrade(&blocks.read()),
            index: 0,
        })
}

fn contains(test: &TestingRunner, caption: &str) -> bool {
    test.find(|_, element| {
        Label::try_downcast(element)
            .filter(|label| label.text.as_ref() == caption)
            .map(|_| ())
            .or_else(|| {
                Paragraph::try_downcast(element)
                    .filter(|paragraph| {
                        paragraph
                            .spans
                            .iter()
                            .any(|span| span.text.as_ref() == caption)
                    })
                    .map(|_| ())
            })
    })
    .is_some()
}
fn click(test: &mut TestingRunner, caption: &str) {
    let area = test
        .find(|node, element| {
            Label::try_downcast(element)
                .filter(|label| label.text.as_ref() == caption)
                .map(|_| node.layout().area)
        })
        .unwrap();
    test.click_cursor(area.center().to_f64());
    test.sync_and_update();
}

#[test]
fn unrelated_parent_redraws_do_not_rescan_large_beats_and_new_snapshots_update_counts() {
    for size in [1_000, 10_000] {
        ENTRIES.set(size);
        ENTRY_VISITS.set(0);
        let mut test = TestingRunner::new(app, (900., 650.).into(), |_| {}, 1.).0;
        test.sync_and_update();
        assert!(contains(&test, "no replies"));
        let initial = ENTRY_VISITS.get();
        assert!(initial >= size, "the first snapshot must be counted");
        for _ in 0..5 {
            click(&mut test, "Redraw");
        }
        assert!(contains(&test, "Revision 5"));
        assert_eq!(
            ENTRY_VISITS.get(),
            initial,
            "parent redraws must not scan the beat"
        );
        click(&mut test, "Replace snapshot");
        assert!(contains(&test, "1 reply"));
        assert_eq!(ENTRY_VISITS.get(), initial + size + 1);
    }
}
