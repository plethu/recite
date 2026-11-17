//! A writer's reading of an effect, built from its parsed call rather than its source text.
use recite_core::{
    ScalarValue,
    ast::{Argument, Effect, EffectMode},
};

#[cfg(test)]
mod tests;

/// An effect as its source line and as a writer reads it, such as "Sets flag (gate_open, true)
/// now" for `! immediate set_flag(gate_open, true)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectEntry {
    pub source: String,
    pub reading: String,
}

/// The call's words with its verb in the third person, its arguments, and when the game receives
/// it: now, now and waiting for the game, or when the scene ends.
pub(super) fn reading(effect: &Effect) -> String {
    let mut words = effect.function.split('_').filter(|word| !word.is_empty());
    let mut reading = words.next().map(third_person).unwrap_or_default();
    for word in words {
        reading.push(' ');
        reading.push_str(word);
    }
    if !effect.args.is_empty() {
        let arguments = effect.args.iter().map(argument).collect::<Vec<_>>();
        reading.push_str(&format!(" ({})", arguments.join(", ")));
    }
    reading.push_str(match effect.mode {
        EffectMode::Immediate => " now",
        EffectMode::Blocking => " now, then waits for the game",
        EffectMode::Deferred => " when the scene ends",
    });
    reading
}

/// `verb` capitalised in the third person singular: "set" reads "Sets", "push" "Pushes", "carry"
/// "Carries".
fn third_person(verb: &str) -> String {
    let mut chars = verb.chars();
    let mut word = chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect::<String>())
        .unwrap_or_default();
    let consonant_y =
        word.ends_with('y') && !word[..word.len() - 1].ends_with(['a', 'e', 'i', 'o', 'u']);
    if consonant_y {
        word.pop();
        word.push_str("ies");
    } else if word.ends_with(['s', 'x', 'z', 'o']) || word.ends_with("ch") || word.ends_with("sh") {
        word.push_str("es");
    } else {
        word.push('s');
    }
    word
}

fn argument(argument: &Argument) -> String {
    match argument {
        Argument::Identifier(name) => name.clone(),
        Argument::Value(ScalarValue::String(text)) => format!("“{text}”"),
        Argument::Value(ScalarValue::Integer(number)) => number.to_string(),
        Argument::Value(ScalarValue::Float(number)) => number.to_string(),
        Argument::Value(ScalarValue::Boolean(value)) => value.to_string(),
    }
}
