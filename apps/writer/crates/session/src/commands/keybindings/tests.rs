use super::*;

#[test]
fn exact_modifiers_case_and_shifted_physical_keys_follow_the_same_policy() {
    let plain = Input {
        key: Key::Character("s"),
        physical: PhysicalKey::Other,
        modifiers: Modifiers {
            primary: true,
            ..Default::default()
        },
    };
    assert!(matches("Primary+S", plain));
    assert!(!matches("Primary+Shift+S", plain));
    assert!(!matches("S", plain));
    assert!(!matches("", plain));
    assert!(!matches(
        "Primary+S",
        Input {
            modifiers: Modifiers {
                other: true,
                ..plain.modifiers
            },
            ..plain
        }
    ));
    let shifted = Input {
        key: Key::Character("!"),
        physical: PhysicalKey::Digit(1),
        modifiers: Modifiers {
            shift: true,
            ..plain.modifiers
        },
    };
    assert!(matches("Primary+Shift+1", shifted));
    assert!(!matches("Primary+Shift+2", shifted));
    assert!(matches(
        "Primary+Shift+,",
        Input {
            key: Key::Character("<"),
            physical: PhysicalKey::Comma,
            ..shifted
        }
    ));
    assert!(matches(
        "F4",
        Input {
            key: Key::Function(4),
            physical: PhysicalKey::Other,
            modifiers: Modifiers::default()
        }
    ));
}

#[test]
fn rebindings_disable_old_keys_and_leave_reserved_editor_keys_local() {
    let mut shortcuts = WriterShortcuts::default();
    let mut input = Input {
        key: Key::Character("s"),
        physical: PhysicalKey::Other,
        modifiers: Modifiers {
            primary: true,
            ..Default::default()
        },
    };
    assert_eq!(command(&shortcuts, input), Some(WriterCommand::Save));
    shortcuts
        .rebind(
            WriterCommand::Save,
            recite_config::WriterShortcut::try_from("Primary+Alt+S".to_owned()).unwrap(),
        )
        .unwrap();
    assert_eq!(command(&shortcuts, input), None);
    input.modifiers.alt = true;
    assert_eq!(command(&shortcuts, input), Some(WriterCommand::Save));
    input.key = Key::Character("z");
    input.modifiers.alt = false;
    assert_eq!(command(&shortcuts, input), None);
    input.key = Key::Character("q");
    assert_eq!(command(&shortcuts, input), None);
}
