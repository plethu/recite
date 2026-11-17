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
/// it: now, now and waiting for the game, or when the scene ends. Written into one string, as a
/// script projects every effect it holds.
pub(super) fn reading(effect: &Effect) -> String {
    let mut reading = String::with_capacity(effect.function.len() + 48);
    let mut words = effect.function.split('_').filter(|word| !word.is_empty());
    if let Some(verb) = words.next() {
        third_person(verb, &mut reading);
    }
    for word in words {
        reading.push(' ');
        reading.push_str(word);
    }
    if !effect.args.is_empty() {
        reading.push_str(" (");
        for (index, value) in effect.args.iter().enumerate() {
            if index > 0 {
                reading.push_str(", ");
            }
            argument(value, &mut reading);
        }
        reading.push(')');
    }
    reading.push_str(match effect.mode {
        EffectMode::Immediate => " now",
        EffectMode::Blocking => " now, then waits for the game",
        EffectMode::Deferred => " when the scene ends",
    });
    reading
}

/// Appends `verb` capitalised in the third person singular: "set" reads "Sets", "push" "Pushes",
/// "carry" "Carries".
fn third_person(verb: &str, out: &mut String) {
    let start = out.len();
    let mut chars = verb.chars();
    if let Some(first) = chars.next() {
        out.extend(first.to_uppercase());
        out.push_str(chars.as_str());
    }
    let word = &out[start..];
    let consonant_y =
        word.ends_with('y') && !word[..word.len() - 1].ends_with(['a', 'e', 'i', 'o', 'u']);
    let sibilant =
        word.ends_with(['s', 'x', 'z', 'o']) || word.ends_with("ch") || word.ends_with("sh");
    if consonant_y {
        out.pop();
        out.push_str("ies");
    } else if sibilant {
        out.push_str("es");
    } else {
        out.push('s');
    }
}

fn argument(argument: &Argument, out: &mut String) {
    use std::fmt::Write as _;
    // Writing into a String cannot fail.
    let _ = match argument {
        Argument::Identifier(name) => out.write_str(name),
        Argument::Value(ScalarValue::String(text)) => write!(out, "“{text}”"),
        Argument::Value(ScalarValue::Integer(number)) => write!(out, "{number}"),
        Argument::Value(ScalarValue::Float(number)) => write!(out, "{number}"),
        Argument::Value(ScalarValue::Boolean(value)) => write!(out, "{value}"),
    };
}
