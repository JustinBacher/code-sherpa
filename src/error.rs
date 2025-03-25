use std::path::PathBuf;

use qdrant_client::QdrantError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to parse file: {0}")]
    ParsingFailed(PathBuf),

    #[error("Invalid Argument: {0}")]
    InvalidArgument(String),

    #[error("Path not found: {0}")]
    NotFound(PathBuf),

    #[error("Scan failed")]
    ScanFailed,

    #[error("Failed to read file: {0}")]
    FileRead(#[from] std::io::Error),

    #[error("Failed to generate embeddings: {0}")]
    Embedding(String),

    #[error(transparent)]
    Storage(#[from] QdrantError),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error("Tree-sitter error: {0}")]
    TreeSitter(#[from] tree_sitter::LanguageError),

    #[error(transparent)]
    Request(#[from] reqwest::Error),

    #[error("Missing {0}")]
    Missing(String),

    #[error(transparent)]
    Ollama(#[from] ollama_rs::error::OllamaError),

    #[error(transparent)]
    UrlParse(#[from] url::ParseError),

    #[error("Unable to serialize payload: {0}")]
    Payload(String),
}
