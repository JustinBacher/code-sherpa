use tree_sitter::{Node, TreeCursor};

/// Preprocesses code to reduce unnecessary content while preserving semantics
pub fn preprocess_code(node: &Node, source: &str) -> String {
    let mut result = String::new();
    let mut cursor = node.walk();

    preprocess_node(&mut cursor, source, &mut result);

    result
}

fn preprocess_node(cursor: &mut TreeCursor, source: &str, result: &mut String) {
    let node = cursor.node();

    // Skip comment nodes entirely
    if node.kind().contains("comment") {
        return;
    }

    // Process node based on its type
    if node.named_child_count() == 0 {
        // Terminal node - include its text with minimal whitespace
        let node_text = node_text(node, source);

        // Preserve certain whitespace but collapse excessive spaces
        let trimmed = normalize_whitespace(node_text);
        result.push_str(&trimmed);
    } else {
        // Non-terminal node - process its children
        if cursor.goto_first_child() {
            preprocess_node(cursor, source, result);

            while cursor.goto_next_sibling() {
                preprocess_node(cursor, source, result);
            }

            cursor.goto_parent();
        }
    }
}

/// Extracts text for a node from source, handling byte offsets correctly
fn node_text<'a>(node: Node, source: &'a str) -> &'a str {
    let start_byte = node.start_byte();
    let end_byte = node.end_byte();

    if end_byte <= source.len() {
        &source[start_byte..end_byte]
    } else {
        // Handle error case gracefully
        ""
    }
}

/// Normalizes whitespace while preserving essential formatting
fn normalize_whitespace(text: &str) -> String {
    // Keep newlines and single spaces, collapse multiple spaces
    let mut result = String::with_capacity(text.len());
    let mut prev_char = ' '; // Start with space to handle leading whitespace

    for c in text.chars() {
        // Keep newlines
        if c == '\n' {
            result.push(c);
            prev_char = c;
            continue;
        }

        // Collapse multiple spaces into one
        if c == ' ' && prev_char == ' ' {
            continue;
        }

        result.push(c);
        prev_char = c;
    }

    result
}
