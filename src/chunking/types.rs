use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct CodeChunk {
    pub content: String,
    pub node_type: String,
    pub start_line: usize,
    pub end_line: usize,
    pub path: PathBuf,
    pub language: String,
}
