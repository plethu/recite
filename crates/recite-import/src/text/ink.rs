use crate::builder::Target;

use super::{Line, Node, plain, unsupported_node};
use crate::{
    ImportError,
    builder::{Builder, identifier},
    diagnostics::UNSUPPORTED,
};

pub(super) fn read(source: &str, builder: &mut Builder) -> Result<(), ImportError> {
    let mut current: Option<Node<'_>> = None;
    for (index, text) in source.lines().enumerate() {
        let line = Line {
            number: index + 1,
            text,
        };
        if let Some(name) = knot(text) {
            if let Some(node) = current.take() {
                emit(node, builder)?;
            }
            current = Some(Node {
                name,
                header: line,
                unsupported_header: false,
                lines: Vec::new(),
            });
        } else if let Some(node) = &mut current {
            node.lines.push(line);
        } else if !text.trim().is_empty() && !text.trim().starts_with("//") {
            builder.issue(UNSUPPORTED, builder.text_provenance(index + 1, text)?, "preamble", "Only named knots are imported; top-level flow and declarations need manual migration.")?;
        }
    }
    if let Some(node) = current {
        emit(node, builder)?;
    }
    Ok(())
}

fn knot(text: &str) -> Option<&str> {
    let value = text.trim().strip_prefix("==")?.trim_matches('=').trim();
    identifier(value).then_some(value)
}

fn emit(node: Node<'_>, builder: &mut Builder) -> Result<(), ImportError> {
    let mut terminal = false;
    let mut choices = false;
    let unsupported = node.lines.iter().any(|line| {
        let text = line.text.trim();
        if text.is_empty() || text.starts_with("//") {
            return false;
        }
        if terminal && !choices {
            return true;
        }
        if choice(text).is_some() {
            terminal = true;
            choices = true;
            return false;
        }
        if choices {
            return true;
        }
        if let Some(target) = text.strip_prefix("-> ") {
            terminal = true;
            return !identifier(target);
        }
        !plain(text)
            || text.starts_with(['*', '+', '-', '~', '=', '!', '&'])
            || text.contains("//")
            || text.contains("/*")
            || ["VAR ", "CONST ", "LIST ", "EXTERNAL ", "INCLUDE ", "TODO:"]
                .iter()
                .any(|prefix| text.starts_with(prefix))
    });
    if unsupported {
        return unsupported_node(
            &node,
            builder,
            "Only plain named knots, static diverts and sticky menu-only choices are supported; once-only choices, expressions, stitches, tags and weave require manual migration.",
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
        if let Some((label, target)) = choice(text) {
            builder.choice(
                label,
                if target == "END" {
                    Target::End
                } else {
                    Target::Block(target)
                },
                provenance,
            )?;
        } else if let Some(target) = text.strip_prefix("-> ") {
            builder.jump(
                if target == "END" {
                    Target::End
                } else {
                    Target::Block(target)
                },
                provenance,
            )?;
        } else {
            builder.line(line.text, None, None, provenance)?;
        }
    }
    Ok(())
}

fn choice(text: &str) -> Option<(&str, &str)> {
    let body = text.strip_prefix("+ [")?;
    let (label, target) = body.split_once("] -> ")?;
    (plain(label) && identifier(target)).then_some((label, target))
}
