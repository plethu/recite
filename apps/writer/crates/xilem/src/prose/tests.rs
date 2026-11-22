use crate::tests::support;
use recite_writer_app::prose::Checker;
use std::fs;
use tokio::sync::mpsc::unbounded_channel;

const SOURCE: &str = ":: start default\n> speaker=rhea @intro\n  The lantern flickerd.\n-> END\n";
const ADD: &str = "Add “flickerd” to the project dictionary";

#[test]
fn prose_findings_are_counted_listed_and_offer_their_fixes()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = support::project(SOURCE)?;
    let manifest = directory.path().join("recite.project.toml");
    let mut text = fs::read_to_string(&manifest)?;
    text.push_str("[prose]\nenabled = true\ndictionaries = [\"words.txt\"]\n");
    fs::write(&manifest, text)?;
    let mut writer = support::open(&directory)?;
    let (sender, mut requests) = unbounded_channel();
    writer.prose.sender = Some(sender);
    writer.check_prose_if_needed();
    let checked = requests.try_recv()?.execute(&mut Checker::default());
    writer.accept_prose(checked);
    let labels = support::labels(&mut writer)?;
    for label in ["1 problem", ADD] {
        assert!(
            labels.iter().any(|name| name == label),
            "{label}: {labels:?}"
        );
    }
    assert!(
        labels
            .iter()
            .any(|name| name.starts_with("Line 3 · flickerd")),
        "{labels:?}"
    );
    assert!(
        labels.iter().any(|name| name.starts_with("Replace with “")),
        "{labels:?}"
    );
    support::click(&mut writer, &[ADD])?;
    assert_eq!(
        fs::read_to_string(directory.path().join("words.txt"))?,
        "flickerd\n"
    );
    Ok(())
}
