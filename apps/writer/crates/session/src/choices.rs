//! Human-readable choices shared by native presentation adapters.
#[derive(Clone, PartialEq)]
pub struct PickerOption {
    pub value: String,
    pub title: String,
    pub detail: String,
    /// Human-facing tag or context, independent of the opaque selection value.
    pub annotation: String,
}

pub fn display_name(identifier: &str) -> String {
    let name = identifier
        .strip_suffix(".recite")
        .unwrap_or(identifier)
        .replace('_', " ");
    name.split(' ')
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().collect::<String>() + chars.as_str()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}
