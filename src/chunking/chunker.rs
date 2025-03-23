use std::{iter::Iterator, path::PathBuf};

use tree_sitter::{Node, Query, QueryCursor, QueryMatches, StreamingIterator, TextProvider, Tree};

use super::types::CodeChunk;

use crate::utils::parsers::SupportedParsers::{self, *};

pub struct Chunker {
    tree: Tree,
    source: String,
    path: PathBuf,
    language: SupportedParsers,
}

impl  Chunker {
    pub fn new(
        tree: &Tree,
        source: &str,
        path: &PathBuf,
        language: &SupportedParsers,
    ) -> Option< Self > {
        let mut chunks = Vec::new();
        let root_node = tree.root_node();

        let Ok(query) = Query::new(
            &language.language(),
            match language {
                Rust => {
                    "(
                    (function_item) @function
                    (struct_item) @struct
                    (impl_item) @impl
                    (trait_item) @trait
                    (enum_item) @enum
                    (macro_definition) @macro
                    )"
                },
                Go => {
                    "(
                    (function_declaration) @function
                    (method_declaration) @method
                    (type_declaration) @type
                    (struct_type) @struct
                    (interface_type) @interface
                    )"
                },
                Python => {
                    "(
                    (function_definition) @function
                    (class_definition) @class
                    (decorated_definition) @decorated
                    )"
                },
                JavaScript | TypeScript | TSX => {
                    "(
                    (function_declaration) @function
                    (method_definition) @method
                    (class_declaration) @class
                    (arrow_function) @arrow
                    )"
                },
            },
        ) else {
            return None;
        };

        let mut query_cursor = QueryCursor::new();
        let mut matches = query_cursor.matches(&query, root_node, source.as_bytes());

        // If no chunks were found (e.g., for small files), create a chunk for the whole file
        if chunks.is_empty() && !source.is_empty() {
            chunks.push(CodeChunk {
                content: source.to_string(),
                node_type: "file".to_string(),
                start_line: 0,
                end_line: root_node.end_position().row,
                path: path.to_path_buf(),
                language: language.to_string(),
            });
        }

        Some(Self {
            tree: tree.clone(),
            source: source.to_string(),
            path: path.to_path_buf(),
            language: *language,
        })
    }
}

impl Iterator for Chunker {
    type Item = u64;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(r#match) = self.matches.next() {h
            for capture in r#match.captures {
                let node = capture.node;
                extract_chunk_from_node(&node, source, path, language, &mut chunks);
            }
        }
    }
}

fn extract_chunk_from_node(
    node: &Node,
    source: &str,
    path: &Path,
    language: &SupportedParsers,
    chunks: &mut Vec<CodeChunk>,
) {
    // Skip nodes that are too small
    if node.start_position().row == node.end_position().row
        && node.end_position().column - node.start_position().column < 10
    {
        return;
    }

    // Extract the chunk text
    let start_byte = node.start_byte();
    let end_byte = node.end_byte();

    if end_byte <= source.len() {
        let chunk_text = &source[start_byte..end_byte];

        chunks.push(CodeChunk {
            content: chunk_text.to_string(),
            node_type: node.kind().to_string(),
            start_line: node.start_position().row,
            end_line: node.end_position().row,
            path: path.to_path_buf(),
            language: language.to_string(),
        });
    }
}
