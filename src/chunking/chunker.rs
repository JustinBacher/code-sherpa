use std::path::{Path, PathBuf};

use tracing::info;
use tree_sitter::{Node, Query, QueryCursor, StreamingIterator, Tree};

use super::preprocess::preprocess_code;
use super::splitter::{add_chunk_context, split_large_chunk};
use super::types::CodeChunk;

use crate::utils::parsers::SupportedParsers;

const DEFAULT_MAX_CHUNK_SIZE: usize = 4096;
const DEFAULT_OVERLAP_PERCENTAGE: usize = 10;

pub struct Chunker {
    tree: Tree,
    source: String,
    path: PathBuf,
    language: SupportedParsers,
    max_chunk_size: usize,
    overlap_percentage: usize,
}

impl Chunker {
    pub fn new(
        tree: &Tree,
        source: &str,
        path: &Path,
        language: &SupportedParsers,
        max_chunk_size: Option<usize>,
        overlap_percentage: Option<usize>,
    ) -> Self {
        Self {
            tree: tree.clone(),
            source: source.to_string(),
            path: path.to_path_buf(),
            language: language.clone(),
            max_chunk_size: max_chunk_size.unwrap_or(DEFAULT_MAX_CHUNK_SIZE),
            overlap_percentage: overlap_percentage.unwrap_or(DEFAULT_OVERLAP_PERCENTAGE),
        }
    }

    pub fn extract_chunks(&self) -> Vec<CodeChunk> {
        let mut chunks = Vec::new();
        let root_node = self.tree.root_node();

        // Execute the query inside this method instead of storing the matches
        let query_result = Query::new(
            &self.language.language(),
            match self.language {
                SupportedParsers::Rust => {
                    "(
                    (function_item) @function
                    (struct_item) @struct
                    (impl_item) @impl
                    (trait_item) @trait
                    (enum_item) @enum
                    (macro_definition) @macro
                    )"
                },
                SupportedParsers::Python => {
                    "(
                    (function_definition) @function
                    (class_definition) @class
                    (decorated_definition) @decorated
                    (if_statement) @if
                    (for_statement) @for
                    (while_statement) @while
                    )"
                },
                SupportedParsers::JavaScript
                | SupportedParsers::TypeScript
                | SupportedParsers::TSX => {
                    "(
                    (function_declaration) @function
                    (method_definition) @method
                    (class_declaration) @class
                    (arrow_function) @arrow_function
                    (variable_declaration (variable_declarator value: (function))) @function_var
                    (variable_declaration (variable_declarator value: (arrow_function))) @arrow_var
                    (export_statement) @export
                    )"
                },
                SupportedParsers::Go => {
                    "(
                    (function_declaration) @function
                    (method_declaration) @method
                    (type_declaration) @type
                    (struct_type) @struct
                    (interface_type) @interface
                    )"
                },
            },
        );

        if let Ok(query) = query_result {
            let mut query_cursor = QueryCursor::new();
            let mut matches = query_cursor.matches(&query, root_node, self.source.as_bytes());

            // Process matches directly here
            while let Some(r#match) = matches.next() {
                for capture in r#match.captures {
                    let node = capture.node;
                    self.extract_chunk_from_node(node, None, &mut chunks);
                }
            }
        }

        // If no chunks were found, create a chunk for the whole file
        if chunks.is_empty() && !self.source.is_empty() {
            chunks.push(CodeChunk {
                content: preprocess_code(&root_node, &self.source),
                node_type: "file".to_string(),
                start_line: 0,
                end_line: root_node.end_position().row,
                path: self.path.clone(),
                language: self.language.to_string(),
            });
        }

        // Split large chunks if needed
        let mut final_chunks = Vec::new();
        for chunk in chunks {
            if chunk.content.len() > self.max_chunk_size {
                final_chunks.extend(split_large_chunk(
                    &chunk,
                    self.max_chunk_size,
                    self.overlap_percentage,
                ));
            } else {
                final_chunks.push(chunk);
            }
        }

        final_chunks
    }

    fn extract_chunk_from_node(
        &self,
        node: Node,
        parent_node: Option<Node>,
        chunks: &mut Vec<CodeChunk>,
    ) {
        // Skip nodes that are too small
        if node.start_position().row == node.end_position().row
            && node.end_position().column - node.start_position().column < 10
        {
            return;
        }

        // Preprocess the node content to remove unnecessary whitespace and comments
        let chunk_text = preprocess_code(&node, &self.source);

        // Create the chunk
        let mut chunk = CodeChunk {
            content: chunk_text,
            node_type: node.kind().to_string(),
            start_line: node.start_position().row,
            end_line: node.end_position().row,
            path: self.path.clone(),
            language: self.language.to_string(),
        };

        // Add context information to the chunk
        add_chunk_context(&mut chunk, node, &self.source, parent_node);

        info!("Extracted a chunk");
        chunks.push(chunk);

        // Process child nodes recursively for nested structures
        let child_count = node.named_child_count();
        for i in 0..child_count {
            if let Some(child) = node.named_child(i) {
                // Skip very small child nodes
                if is_significant_node(&child) {
                    self.extract_chunk_from_node(child, Some(node), chunks);
                }
            }
        }
    }
}

/// Check if a node is significant enough to be processed as a chunk
fn is_significant_node(node: &Node) -> bool {
    let node_type = node.kind();

    // Skip comment nodes
    if node_type.contains("comment") {
        return false;
    }

    // Skip nodes that are too small
    if node.start_position().row == node.end_position().row
        && node.end_position().column - node.start_position().column < 20
    {
        return false;
    }

    matches!(
        node_type,
        "function_item"
            | "method"
            | "class"
            | "struct_item"
            | "impl_item"
            | "trait_item"
            | "enum_item"
            | "type_declaration"
            | "function_declaration"
            | "method_declaration"
            | "class_declaration"
            | "function_definition"
            | "class_definition"
            | "interface_type"
            | "decorated_definition"
            | "arrow_function"
    )
}

/// Extract chunks from a tree-sitter parse tree
pub fn extract_chunks(
    tree: &Tree,
    source: &str,
    path: &Path,
    language: &SupportedParsers,
    max_chunk_size: Option<usize>,
    overlap_percentage: Option<usize>,
) -> Vec<CodeChunk> {
    Chunker::new(
        tree,
        source,
        path,
        language,
        max_chunk_size,
        overlap_percentage,
    )
    .extract_chunks()
}
