use tree_sitter::Node;

use crate::chunking::CodeChunk;

/// Split large chunks into smaller ones with semantic boundaries and overlap
pub fn split_large_chunk(
    chunk: &CodeChunk,
    max_size: usize,
    overlap_percentage: usize,
) -> Vec<CodeChunk> {
    if chunk.content.len() <= max_size {
        return vec![chunk.clone()];
    }

    // Calculate overlap size (in bytes)
    let overlap_size = (max_size * overlap_percentage) / 100;
    let effective_chunk_size = max_size - overlap_size;

    // Split content along semantic boundaries
    let mut chunks = Vec::new();
    let mut current_pos = 0;

    while current_pos < chunk.content.len() {
        let end_pos =
            find_semantic_boundary(&chunk.content, current_pos + effective_chunk_size, max_size);

        // Create a new chunk with the split content
        let split_content = &chunk.content[current_pos..end_pos];

        // Calculate line numbers for the split chunk
        let start_line_offset = count_lines(&chunk.content[0..current_pos]);
        let chunk_lines = count_lines(split_content);

        chunks.push(CodeChunk {
            content: split_content.to_string(),
            node_type: format!("{}_part", chunk.node_type),
            start_line: chunk.start_line + start_line_offset,
            end_line: chunk.start_line + start_line_offset + chunk_lines,
            path: chunk.path.clone(),
            language: chunk.language.clone(),
        });

        // Move position with overlap
        current_pos = if end_pos >= chunk.content.len() {
            chunk.content.len()
        } else {
            end_pos - overlap_size
        };
    }

    chunks
}

fn find_semantic_boundary(content: &str, target_pos: usize, max_pos: usize) -> usize {
    let end_pos = std::cmp::min(target_pos, content.len());
    let max_pos = std::cmp::min(max_pos, content.len());

    // Ensure end_pos doesn't exceed max_pos
    let end_pos = std::cmp::min(end_pos, max_pos);

    // Try to find a good boundary (blank line, end of statement, etc.)
    let search_range = &content[end_pos..max_pos];

    // Look for semantic boundaries in priority order:

    // 1. Blank line (two consecutive newlines)
    if let Some(pos) = search_range.find("\n\n") {
        return end_pos + pos + 2;
    }

    // 2. Line end (single newline)
    if let Some(pos) = search_range.find('\n') {
        return end_pos + pos + 1;
    }

    // 3. Statement end (semicolon, curly brace, etc.)
    for boundary in [";", "}", "{"] {
        if let Some(pos) = search_range.find(boundary) {
            return end_pos + pos + boundary.len();
        }
    }

    // 4. Fall back to a space character
    if let Some(pos) = search_range.find(' ') {
        return end_pos + pos + 1;
    }

    // If no boundary found, just use the end_pos position
    end_pos
}

/// Count the number of lines in a string
fn count_lines(content: &str) -> usize {
    content.matches('\n').count()
}

/// Add context to chunks to improve embedding quality
pub fn add_chunk_context(
    chunk: &mut CodeChunk,
    node: Node,
    source: &str,
    parent_node: Option<Node>,
) {
    // Extract node name if available
    if let Some(name_node) = find_node_name(node) {
        if let Some(name) = node_text(name_node, source) {
            chunk.node_type = format!("{}:{}", chunk.node_type, name);
        }
    }

    // Add parent context if available
    if let Some(parent) = parent_node {
        if let Some(parent_name_node) = find_node_name(parent) {
            if let Some(parent_name) = node_text(parent_name_node, source) {
                chunk.content = format!(
                    "// In {}: {}\n{}",
                    parent.kind(),
                    parent_name,
                    chunk.content
                );
            }
        }
    }
}

fn find_node_name(node: Node) -> Option<Node> {
    // Different node types store their names in different child nodes
    let child_count = node.named_child_count();
    for i in 0..child_count {
        let child = node.named_child(i)?;
        let child_type = child.kind();

        // Common name patterns
        if child_type == "identifier"
            || child_type == "name"
            || child_type.ends_with("_name")
            || child_type.ends_with("_identifier")
        {
            return Some(child);
        }
    }

    None
}

/// Extract text for a node from source
fn node_text<'a>(node: Node, source: &'a str) -> Option<&'a str> {
    let end_byte = node.end_byte();

    if end_byte <= source.len() {
        Some(&source[node.start_byte()..end_byte])
    } else {
        None
    }
}
