use crate::types::Position;
use tree_sitter::Node;

pub fn node_to_position(node: &Node, start: bool) -> Position {
    let point = if start {
        node.start_position()
    } else {
        node.end_position()
    };
    Position {
        line: point.row as u32,
        character: point.column as u32,
    }
}

pub fn find_doc_comments_before(node: &Node, source: &str) -> Option<(Position, Position)> {
    let mut current = node.prev_sibling();
    let mut first_doc_line = None;
    let mut last_doc_line = None;

    while let Some(sibling) = current {
        if sibling.kind() == "line_comment" {
            let comment_text = sibling.utf8_text(source.as_bytes()).ok()?;
            if comment_text.starts_with("///") {
                let start_pos = node_to_position(&sibling, true);
                if first_doc_line.is_none() {
                    first_doc_line = Some(start_pos);
                }
                last_doc_line = Some(start_pos);
            } else {
                break;
            }
        } else if sibling.kind() != "attribute_item" {
            break;
        }
        current = sibling.prev_sibling();
    }

    if let (Some(first), Some(_)) = (first_doc_line, last_doc_line) {
        let end = node_to_position(node, false);
        Some((first, end))
    } else {
        None
    }
}
