#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct GlobPattern {
    segments: Vec<String>,
}

impl GlobPattern {
    pub(super) fn parse(source: &str) -> Result<Self, String> {
        validate_relative_pattern(source)?;
        Ok(Self {
            segments: normalize_relative_pattern(source)
                .split('/')
                .map(str::to_owned)
                .collect(),
        })
    }

    pub(super) fn matches(&self, path: &str) -> bool {
        let path = path.split('/').collect::<Vec<_>>();
        let mut previous = vec![false; path.len() + 1];
        previous[0] = true;
        for segment in &self.segments {
            let mut next = vec![false; path.len() + 1];
            if segment == "**" {
                next[0] = previous[0];
                for index in 1..=path.len() {
                    next[index] = previous[index] || next[index - 1];
                }
            } else {
                for index in 1..=path.len() {
                    next[index] = previous[index - 1] && wildcard_match(segment, path[index - 1]);
                }
            }
            previous = next;
        }
        previous[path.len()]
    }
}

pub(super) fn normalize_relative_pattern(value: &str) -> String {
    let components = value.split('/').filter(|component| *component != ".");
    let normalized = components.collect::<Vec<_>>().join("/");
    if normalized.is_empty() {
        ".".to_owned()
    } else {
        normalized
    }
}

pub(super) fn validate_relative_pattern(value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err("pattern cannot be empty".to_owned());
    }
    if value.starts_with('/') || value.starts_with('\\') {
        return Err("pattern must be project-relative".to_owned());
    }
    if value.len() >= 2 && value.as_bytes()[0].is_ascii_alphabetic() && value.as_bytes()[1] == b':'
    {
        return Err("absolute Windows-shaped paths are not allowed".to_owned());
    }
    if value.contains('\\') {
        return Err("use slash-separated project-relative paths".to_owned());
    }
    if value.starts_with('!') {
        return Err("negation patterns are not allowed".to_owned());
    }
    if value.split('/').any(|segment| segment == "..") {
        return Err("parent components are not allowed".to_owned());
    }
    if value.split('/').any(str::is_empty) {
        return Err("empty path components are not allowed".to_owned());
    }
    Ok(())
}

fn wildcard_match(pattern: &str, value: &str) -> bool {
    let pattern = pattern.chars().collect::<Vec<_>>();
    let value = value.chars().collect::<Vec<_>>();
    let mut previous = vec![false; value.len() + 1];
    previous[0] = true;
    for token in pattern {
        let mut next = vec![false; value.len() + 1];
        if token == '*' {
            next[0] = previous[0];
            for index in 1..=value.len() {
                next[index] = previous[index] || next[index - 1];
            }
        } else {
            for index in 1..=value.len() {
                next[index] = previous[index - 1] && (token == '?' || token == value[index - 1]);
            }
        }
        previous = next;
    }
    previous[value.len()]
}
