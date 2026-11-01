use super::Icon;
use crate::presentation::{self, print};
use crate::{
    keyed::keyed,
    tests::support::{mounted::Mounted, native_label},
};
use xilem::{
    WidgetView,
    masonry::widgets::{Button, Passthrough},
};

#[test]
fn icon_faces_keep_the_native_button_name_and_activation() {
    fn page(state: &mut bool) -> impl WidgetView<bool, Widget = Passthrough> + use<> {
        let _ = state;
        keyed(
            (),
            print::icon_action(
                1.,
                "Back",
                Icon::Back,
                &presentation::palette(&recite_config::WriterConfig::default()),
                false,
                |state: &mut bool| *state = true,
            ),
        )
    }
    let mut mounted = Mounted::new(false, page);
    let id = mounted.find("Back");
    assert!(
        mounted
            .harness
            .get_widget_with_id(id)
            .downcast::<Button>()
            .is_some()
    );
    assert_eq!(native_label(&mounted.harness, id).as_deref(), Some("Back"));
    mounted.activate("Back");
    assert!(mounted.state);
}
