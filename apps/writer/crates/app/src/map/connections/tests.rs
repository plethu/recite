use super::*;
use recite_writer_model::SceneLink;

#[test]
fn directions_name_their_end_of_a_link() {
    let link = SceneLink {
        origin: "start".into(),
        destination: "end".into(),
        label: String::new(),
        conditional: false,
        returning: false,
    };
    assert_eq!(Direction::Outgoing.target(&link), "end");
    assert_eq!(Direction::Incoming.target(&link), "start");
    assert_eq!(Direction::Outgoing.label(), "Outgoing");
    assert_eq!(Direction::Incoming.label(), "Incoming");
}

#[test]
fn each_direction_keeps_its_own_page() {
    let mut connections = Connections::default();
    connections.list_mut(Direction::Incoming).page = 2;
    assert_eq!(connections.list(Direction::Incoming).page, 2);
    assert_eq!(connections.list(Direction::Outgoing).page, 0);
}
