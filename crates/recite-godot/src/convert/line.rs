use godot::builtin::{VarArray, VarDictionary, Variant};
use godot::prelude::ToGodot;

use recite_runtime::{ChoiceEchoMode, DialogueChoice, DialogueLine};

use super::core::{push_variant, set_variant};
use super::effects::metadata_array;
use super::plural::plural_dictionary;
use super::reason::availability_dictionary;

pub(super) fn line_dictionary(line: &DialogueLine) -> VarDictionary {
    let mut dictionary = VarDictionary::new();
    dictionary.set("id", line.id.as_str());
    dictionary.set("source_text", line.source_text.as_str());
    dictionary.set("text", line.text.as_str());
    set_variant(
        &mut dictionary,
        "speaker",
        line.speaker
            .as_ref()
            .map_or_else(Variant::nil, |speaker| speaker.as_str().to_variant()),
    );
    set_variant(
        &mut dictionary,
        "metadata",
        metadata_array(&line.metadata).to_variant(),
    );
    if let Some(plural) = line.plural.as_ref() {
        set_variant(
            &mut dictionary,
            "plural",
            plural_dictionary(plural).to_variant(),
        );
    }
    dictionary
}

pub(super) fn choices_array(choices: &[DialogueChoice]) -> VarArray {
    let mut array = VarArray::new();
    for choice in choices {
        push_variant(&mut array, choice_dictionary(choice).to_variant());
    }
    array
}

fn choice_dictionary(choice: &DialogueChoice) -> VarDictionary {
    let mut dictionary = VarDictionary::new();
    dictionary.set("id", choice.id.as_str());
    dictionary.set("source_text", choice.source_text.as_str());
    dictionary.set("text", choice.text.as_str());
    set_variant(
        &mut dictionary,
        "metadata",
        metadata_array(&choice.metadata).to_variant(),
    );
    set_variant(
        &mut dictionary,
        "availability",
        availability_dictionary(&choice.availability).to_variant(),
    );
    let (echo, line_id) = match &choice.echo {
        ChoiceEchoMode::None => ("none", None),
        ChoiceEchoMode::SelectedText => ("selected_text", None),
        ChoiceEchoMode::ExplicitLine(id) => ("explicit_line", Some(id.as_str())),
    };
    dictionary.set("echo", echo);
    set_variant(
        &mut dictionary,
        "echo_line_id",
        line_id.map_or_else(Variant::nil, |line_id| line_id.to_variant()),
    );
    dictionary
}
