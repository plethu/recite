use super::*;
use crate::{
    keyed::keyed,
    tests::support::{mounted::Mounted, open, project},
};
use recite_config::WriterPresentationField;
use xilem::masonry::widgets::Passthrough;

fn surface(writer: &mut Writer) -> impl WidgetView<Writer, Widget = Passthrough> + use<> {
    keyed((), crate::view::writer(writer))
}

#[test]
fn revealing_a_long_reply_label_never_moves_the_following_reply() {
    let source = format!(
        ":: start default\n> greeting@11111111111111111111\n  Hello.\n? {}@22222222222222222222\n  First reply\n  -> next\n? other@33333333333333333333\n  Second reply\n  -> next\n:: next\n> end@44444444444444444444\n  Goodbye.\n-> END\n",
        "long_source_label_".repeat(12)
    );
    let directory = project(&source).unwrap();
    let mut writer = open(&directory).unwrap();
    let mut config = writer.config.clone();
    config.writer.presentation = config
        .writer
        .presentation
        .with_value(WriterPresentationField::UiScale, 200)
        .unwrap();
    writer.configure(config);
    let mut mounted = Mounted::new(writer, surface);
    let following = mounted.find("Edit passage 3");
    let before = mounted
        .harness
        .get_widget_with_id(following)
        .ctx()
        .window_transform()
        .translation()
        .y;
    mounted.state.change(|model| {
        model.select(View::Passage("22222222222222222222".into()))?;
        model.set_draft("Edited first reply".into());
        Ok(())
    });
    mounted.settle();
    assert_eq!(mounted.find("Edit passage 3"), following);
    let after = mounted
        .harness
        .get_widget_with_id(following)
        .ctx()
        .window_transform()
        .translation()
        .y;
    assert_eq!(
        before, after,
        "source identity must share the destination row's fixed space"
    );
}
