use std::path::{Path, PathBuf};

use tracing::{debug, info, warn};
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
        debug!("Extracting chunks from {}", self.path.display());

        let structured_chunks = self.extract_structured_chunks(root_node);
        if !structured_chunks.is_empty() {
            chunks.extend(structured_chunks);
        }

        if chunks.is_empty() {
            chunks.extend(self.extract_general_chunks(root_node));
        }

        // If we still have no chunks, use the whole file
        if chunks.is_empty() && !self.source.is_empty() {
            debug!(
                "No specific chunks found, using entire file: {}",
                self.path.display()
            );
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
                let split = split_large_chunk(&chunk, self.max_chunk_size, self.overlap_percentage);
                final_chunks.extend(split);
            } else {
                final_chunks.push(chunk);
            }
        }

        debug!(
            "Extracted {} chunks from {}",
            final_chunks.len(),
            self.path.display()
        );
        final_chunks
    }

    // Extract chunks using structured, language-specific queries
    fn extract_structured_chunks(&self, root_node: Node) -> Vec<CodeChunk> {
        let mut chunks = Vec::new();

        let query_str = match self.language {
            SupportedParsers::Rust => {
                "(
                (function_item) @function
                (struct_item) @struct
                (impl_item
                    body: (declaration_list
                        (function_item) @impl_method))
                (trait_item
                    body: (declaration_list
                        (function_item) @trait_method))
                (enum_item) @enum
                (mod_item) @mod
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
            SupportedParsers::JavaScript | SupportedParsers::TypeScript | SupportedParsers::TSX => {
                "(
                (function_declaration) @function
                (method_definition) @method
                (class_declaration) @class
                (arrow_function) @arrow_function
                (export_statement) @export
                (lexical_declaration) @declaration
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
        };

        // Execute the query
        match Query::new(&self.language.language(), query_str) {
            Ok(query) => {
                let mut query_cursor = QueryCursor::new();
                let mut matches = query_cursor.matches(&query, root_node, self.source.as_bytes());

                // Process each match directly - no recursion
                while let Some(match_result) = matches.next() {
                    for capture in match_result.captures {
                        let node = capture.node;
                        let kind = node.kind();

                        // Skip very small nodes
                        if node.start_position().row == node.end_position().row
                            && node.end_position().column - node.start_position().column < 3
                        {
                            continue;
                        }

                        info!("Kind: {}", kind);
                        // Capture child chunks
                        if kind == "impl_item" || kind == "trait_item" {
                            self.extract_structured_chunks(node);
                        }

                        // Create the chunk
                        let mut chunk = CodeChunk {
                            content: preprocess_code(&node, &self.source),
                            node_type: kind.to_string(),
                            start_line: node.start_position().row,
                            end_line: node.end_position().row,
                            path: self.path.clone(),
                            language: self.language.to_string(),
                        };

                        add_chunk_context(&mut chunk, node, &self.source, node.parent());

                        chunks.push(chunk);
                    }
                }
            },
            Err(e) => {
                warn!(
                    "Failed to create query for language {:?}: {}",
                    self.language, e
                );
            },
        }

        chunks
    }

    // Extract chunks using a general approach when language-specific queries fail
    fn extract_general_chunks(&self, root_node: Node) -> Vec<CodeChunk> {
        let mut chunks = Vec::new();

        // Use a generic query to find blocks and statements
        let general_query = "(
            (block) @block
            (statement) @statement
            (declaration) @declaration
        )";

        match Query::new(&self.language.language(), general_query) {
            Ok(query) => {
                let mut query_cursor = QueryCursor::new();
                let mut matches = query_cursor.matches(&query, root_node, self.source.as_bytes());

                while let Some(match_result) = matches.next() {
                    for capture in match_result.captures {
                        let node = capture.node;
                        info!("General Kind: {}", node.kind());

                        // Only consider substantial blocks
                        if node.end_position().row - node.start_position().row < 3 {
                            continue;
                        }

                        // Create the chunk
                        let chunk_text = preprocess_code(&node, &self.source);
                        chunks.push(CodeChunk {
                            content: chunk_text,
                            node_type: node.kind().to_string(),
                            start_line: node.start_position().row,
                            end_line: node.end_position().row,
                            path: self.path.clone(),
                            language: self.language.to_string(),
                        });
                    }
                }
            },
            Err(_) => {
                // If even general query fails, try line-based chunking
                debug!(
                    "Falling back to section-based chunking for {}",
                    self.path.display()
                );
                chunks.extend(self.extract_section_chunks());
            },
        }

        chunks
    }

    // Extract chunks based on natural sections in the code
    fn extract_section_chunks(&self) -> Vec<CodeChunk> {
        let mut chunks = Vec::new();
        let lines: Vec<&str> = self.source.lines().collect();

        if lines.is_empty() {
            return chunks;
        }

        let mut section_start = 0;
        let mut blank_line_count = 0;
        let mut in_comment_block = false;

        // Look for natural sections separated by blank lines or comment blocks
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Track comment blocks
            if trimmed.starts_with("/*") || trimmed.starts_with("/**") {
                in_comment_block = true;
            }
            if trimmed.ends_with("*/") {
                in_comment_block = false;
                blank_line_count = 0;
                continue;
            }

            // Count consecutive blank lines
            if trimmed.is_empty() && !in_comment_block {
                blank_line_count += 1;
            } else {
                blank_line_count = 0;
            }

            // Start a new section after 2+ blank lines or at substantial section size
            if (blank_line_count >= 2 && i - section_start > 5) || i - section_start >= 100 {
                let section_content = lines[section_start..i].join("\n");
                if !section_content.trim().is_empty() {
                    chunks.push(CodeChunk {
                        content: section_content,
                        node_type: "section".to_string(),
                        start_line: section_start,
                        end_line: i,
                        path: self.path.clone(),
                        language: self.language.to_string(),
                    });
                }
                section_start = i + 1;
                blank_line_count = 0;
            }
        }

        // Add the last section
        if section_start < lines.len() {
            let section_content = lines[section_start..].join("\n");
            if !section_content.trim().is_empty() {
                chunks.push(CodeChunk {
                    content: section_content,
                    node_type: "section".to_string(),
                    start_line: section_start,
                    end_line: lines.len(),
                    path: self.path.clone(),
                    language: self.language.to_string(),
                });
            }
        }

        chunks
    }
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
