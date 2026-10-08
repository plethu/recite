use super::*;
use freya_testing::prelude::*;
use recite_writer_model::View;

const SOURCE: &str = ":: start default\n> line@11111111111111111111\n  Saved prose.\n-> END\n";
const DRAFT: &str = ":: start default\n> line@11111111111111111111\n  Unfinished prose.\n-> END\n";

#[derive(Clone)]
struct Scenario {
    root: std::path::PathBuf,
    save: bool,
}

fn durability() -> impl IntoElement {
    use_init_theme(light_theme);
    let scenario = consume_root_context::<Scenario>();
    let mut files = use_state(move || Some(ProjectFiles::open(&scenario.root).unwrap()));
    let model = use_state(move || {
        let mut model = files.write().as_mut().unwrap().workbench().unwrap();
        model.select(View::Source).unwrap();
        Ok(model)
    });
    let buffers = Buffers {
        bookmarks: crate::buffers::Bookmarks::new(),
        rules: use_state(|| None),
        model,
        editor: use_state(|| crate::editing::editor_data(DRAFT, true, false)),
        prose: use_state(String::new),
    };
    let localisation = use_state(crate::localisation::Localisation::default);
    let mut status = use_state(String::new);
    rect()
        .child(
            crate::design::Button::new()
                .named("Flush before close")
                .child("Flush before close")
                .on_press(move |_| {
                    let result = if scenario.save {
                        buffers.save_all(files)
                    } else {
                        Ok(())
                    }
                    .and_then(|()| flush_recovery(buffers, files, localisation));
                    status.set(result.map_or_else(|error| error, |()| "Durable".into()));
                }),
        )
        .child(label().text(status.read().clone()))
}

#[test]
fn close_durability_preserves_recovery_or_saves_source_before_clearing_it()
-> Result<(), Box<dyn std::error::Error>> {
    for save in [false, true] {
        let dir = tempfile::tempdir()?;
        std::fs::write(
            dir.path().join("recite.project.toml"),
            "format_version = 1\n[project]\n",
        )?;
        let path = dir.path().join("scene.recite");
        std::fs::write(&path, SOURCE)?;
        let scenario = Scenario {
            root: dir.path().to_owned(),
            save,
        };
        let mut test = TestingRunner::new(
            durability,
            (700., 300.).into(),
            move |runner| {
                runner.provide_root_context(move || scenario);
            },
            1.,
        )
        .0;
        let area = test
            .find(|node, element| {
                Rect::try_downcast(element)
                    .filter(|rect| rect.accessibility.builder.label() == Some("Flush before close"))
                    .map(|_| node.layout().area)
            })
            .ok_or("durability action")?;
        test.click_cursor(area.center().to_f64());
        test.sync_and_update();
        assert!(
            test.find(|_, element| Label::try_downcast(element)
                .filter(|label| label.text.as_ref() == "Durable"))
                .is_some()
        );
        assert_eq!(
            std::fs::read_to_string(&path)?,
            if save { DRAFT } else { SOURCE }
        );
        drop(test);
        let mut reopened = ProjectFiles::open(dir.path())?;
        let mut recovered = reopened.workbench()?;
        if save {
            recovered.select(View::Source)?;
        } else {
            assert_eq!(recovered.view(), &View::Source);
        }
        assert_eq!(recovered.draft(), DRAFT);
        assert_eq!(reopened.has_recovery(), !save);
    }
    Ok(())
}
