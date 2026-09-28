use crate::builder::Target;

use super::{Line, Node, plain, unsupported_node};
use crate::{ImportError, builder::Builder, diagnostics::UNSUPPORTED};

pub(super) fn read(source: &str, builder: &mut Builder) -> Result<(), ImportError> {
    let mut current: Option<Node<'_>> = None;
    for (index, text) in source.lines().enumerate() {
        let line = Line {
            number: index + 1,
            text,
        };
        if let Some(name) = text.strip_prefix(":: ") {
            if let Some(node) = current.take() {
                emit(node, builder)?;
            }
            current = Some(Node {
                name: name.trim(),
                header: line,
                unsupported_header: false,
                lines: Vec::new(),
            });
        } else if let Some(node) = &mut current {
            node.lines.push(line);
        } else if !text.trim().is_empty() {
            builder.issue(
                UNSUPPORTED,
                builder.text_provenance(index + 1, text)?,
                "preamble",
                "Text outside a Twee passage.",
            )?;
        }
    }
    if let Some(node) = current {
        emit(node, builder)?;
    }
    Ok(())
}

fn emit(node: Node<'_>, builder: &mut Builder) -> Result<(), ImportError> {
    // StoryData, scripts and styles are never emitted as dialogue. Header tags
    // and metadata require author mapping and therefore hold back the passage.
    let reserved = matches!(
        node.name,
        "StoryData" | "StoryTitle" | "StoryInit" | "StoryCaption"
    );
    let mut links = false;
    let unsupported = reserved
        || node.name.contains(['[', ']', '{', '}', '\\'])
        || node.lines.iter().any(|line| {
            let text = line.text.trim();
            if text.is_empty() {
                return false;
            }
            if link(text).is_some() {
                links = true;
                false
            } else {
                links || !plain(text) || text.contains(['(', ')', '|', '='])
            }
        });
    if unsupported {
        return unsupported_node(
            &node,
            builder,
            "Only plain passages followed by standalone static links are supported; story data, tags, macros and formatting need manual migration.",
        );
    }
    builder.block(
        node.name,
        builder.text_provenance(node.header.number, node.header.text)?,
    )?;
    for line in node.lines {
        let text = line.text.trim();
        if text.is_empty() {
            continue;
        }
        let provenance = builder.text_provenance(line.number, line.text)?;
        if let Some((label, target)) = link(text) {
            builder.choice(label, Target::Block(target), provenance)?;
        } else {
            builder.line(text, None, None, provenance)?;
        }
    }
    Ok(())
}

fn link(text: &str) -> Option<(&str, &str)> {
    let inner = text.strip_prefix("[[")?.strip_suffix("]]")?;
    if inner.contains(['[', ']', '{', '}', '$', '(', ')']) {
        return None;
    }
    let (label, target) = if let Some(parts) = inner.split_once("->") {
        parts
    } else if let Some((target, label)) = inner.split_once("<-") {
        (label, target)
    } else {
        inner.split_once('|').unwrap_or((inner, inner))
    };
    if !plain(label.trim()) || target.contains(['|', '<', '>']) || target.trim().is_empty() {
        return None;
    }
    Some((label.trim(), target.trim()))
}
