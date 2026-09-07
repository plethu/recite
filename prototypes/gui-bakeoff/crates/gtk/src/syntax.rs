use gtk::prelude::*;
use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};

pub fn highlight(view: &gtk::TextView, source: bool, dark: bool) {
    if let Err(error) = apply(view, source, dark) {
        eprintln!("Source highlighting unavailable: {error}");
    }
}

fn apply(view: &gtk::TextView, source: bool, dark: bool) -> Result<(), Box<dyn std::error::Error>> {
    let buffer = view.buffer();
    buffer.remove_all_tags(&buffer.start_iter(), &buffer.end_iter());
    if !source {
        return Ok(());
    }
    let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), true);
    let language = recite_bakeoff_grammar::LANGUAGE.into();
    let mut parser = Parser::new();
    parser.set_language(&language)?;
    let tree = parser
        .parse(text.as_str(), None)
        .ok_or("parser cancelled")?;
    let query = Query::new(&language, recite_bakeoff_grammar::HIGHLIGHTS)?;
    let mut cursor = QueryCursor::new();
    let mut captures = cursor.captures(&query, tree.root_node(), text.as_bytes());
    while let Some((matched, index)) = captures.next() {
        let capture = matched.captures[*index];
        let name = query.capture_names()[capture.index as usize];
        let (light, night) = match name.split('.').next().unwrap_or(name) {
            "keyword" | "punctuation" | "operator" => ("#365f92", "#a6c8f0"),
            "label" | "variable" => ("#72528b", "#d2b5ec"),
            "function" | "type" | "tag" => ("#266967", "#8acfc6"),
            "constant" | "number" | "boolean" => ("#865520", "#e6be85"),
            "comment" | "property" => ("#65615b", "#bcb8b1"),
            "error" => ("#963f36", "#f2b3a6"),
            "string" if name != "string.special" => ("#865520", "#e6be85"),
            _ => ("#322f2b", "#eae7e1"),
        };
        let table = buffer.tag_table();
        let tag = table.lookup(name).unwrap_or_else(|| {
            let tag = gtk::TextTag::builder().name(name).build();
            table.add(&tag);
            tag
        });
        tag.set_foreground(Some(if dark { night } else { light }));
        let start = i32::try_from(text[..capture.node.start_byte()].chars().count())?;
        let end = i32::try_from(text[..capture.node.end_byte()].chars().count())?;
        buffer.apply_tag(
            &tag,
            &buffer.iter_at_offset(start),
            &buffer.iter_at_offset(end),
        );
    }
    Ok(())
}
