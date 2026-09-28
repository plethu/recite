use crate::builder::Target;

use super::{Line, Node, plain, plain_character, unsupported_node};
use crate::{
    ImportError,
    builder::{Builder, identifier, speaker_value},
    diagnostics::INVALID,
};

enum State<'a> {
    Header {
        title: Option<Line<'a>>,
        extra: Vec<Line<'a>>,
    },
    Body(Node<'a>),
}

pub(super) fn read(source: &str, builder: &mut Builder) -> Result<(), ImportError> {
    let mut state = State::Header {
        title: None,
        extra: Vec::new(),
    };
    for (index, text) in source.lines().enumerate() {
        let line = Line {
            number: index + 1,
            text,
        };
        state = match state {
            State::Header {
                mut title,
                mut extra,
            } => {
                if text == "---" {
                    if let Some(header) = title {
                        State::Body(Node {
                            name: header.text.strip_prefix("title:").unwrap_or("").trim(),
                            header,
                            unsupported_header: !extra.is_empty(),
                            lines: extra,
                        })
                    } else {
                        builder.issue(
                            INVALID,
                            builder.text_provenance(index + 1, text)?,
                            "node",
                            "Yarn node is missing a title.",
                        )?;
                        State::Header { title: None, extra }
                    }
                } else {
                    if text.starts_with("title:") && title.is_none() {
                        title = Some(line);
                    } else if !text.trim().is_empty() {
                        extra.push(line);
                    }
                    State::Header { title, extra }
                }
            }
            State::Body(mut node) => {
                if text == "===" {
                    emit(node, builder)?;
                    State::Header {
                        title: None,
                        extra: Vec::new(),
                    }
                } else {
                    node.lines.push(line);
                    State::Body(node)
                }
            }
        };
    }
    match state {
        State::Header { title: None, extra } if extra.is_empty() => Ok(()),
        _ => builder.issue(
            INVALID,
            builder.provenance(crate::Location::Document, "document"),
            "node",
            "Yarn node is incomplete; expected title, ---, body and ===.",
        ),
    }
}

fn emit(node: Node<'_>, builder: &mut Builder) -> Result<(), ImportError> {
    let mut terminal = false;
    let unsupported = node.unsupported_header
        || !identifier(node.name)
        || node.lines.iter().any(|line| {
            let text = line.text.trim();
            if text.is_empty() || text.starts_with("//") {
                return false;
            }
            if terminal {
                return true;
            }
            if jump(text).is_some() {
                terminal = true;
                return false;
            }
            let (text, _) = line_id(text);
            let (speaker, body) = speaker_prefix(text);
            !plain(body)
                || speaker.is_some_and(|speaker| {
                    !speaker_value(speaker)
                        || speaker.contains("''")
                        || !speaker.chars().all(|c| c == '_' || plain_character(c))
                })
                || line.text.starts_with(char::is_whitespace)
                || text.starts_with("->")
                || text.contains("//")
        });
    if unsupported {
        return unsupported_node(
            &node,
            builder,
            "Only plain nodes, speaker prefixes, line IDs and static jumps are supported; options, commands, expressions, tags and custom headers require manual migration.",
        );
    }
    builder.block(
        node.name,
        builder.text_provenance(node.header.number, node.header.text)?,
    )?;
    for line in node.lines {
        let text = line.text.trim();
        if text.is_empty() || text.starts_with("//") {
            continue;
        }
        let provenance = builder.text_provenance(line.number, line.text)?;
        if let Some(target) = jump(text) {
            builder.jump(Target::Block(target), provenance)?;
        } else {
            let (text, id) = line_id(text);
            let (speaker, text) = speaker_prefix(text);
            builder.line(text, speaker, id, provenance)?;
        }
    }
    Ok(())
}

fn speaker_prefix(text: &str) -> (Option<&str>, &str) {
    text.split_once(": ")
        .map_or((None, text), |(speaker, text)| (Some(speaker), text))
}

fn jump(text: &str) -> Option<&str> {
    let target = text.strip_prefix("<<jump ")?.strip_suffix(">>")?.trim();
    identifier(target).then_some(target)
}

fn line_id(text: &str) -> (&str, Option<&str>) {
    text.rsplit_once(" #line:")
        .filter(|(_, id)| !id.is_empty() && !id.contains(char::is_whitespace))
        .map_or((text, None), |(text, id)| (text, Some(id)))
}
