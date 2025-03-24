use std::path::PathBuf;

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

    #[error("Unsupported language: {0}")]
    UnsupportedLanguage(String),

    #[error("Failed to read file: {0}")]
    FileReadError(#[from] std::io::Error),

    #[error("Failed to generate embeddings: {0}")]
    EmbeddingError(String),

    #[error("Failed to store embeddings: {0}")]
    StorageError(String),

    #[error("Tree-sitter error: {0}")]
    TreeSitterError(#[from] tree_sitter::LanguageError),

    #[error("Request error: {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("Missing {0}")]
    Missing(String),

    #[error(transparent)]
    OllamaError(#[from] ollama_rs::error::OllamaError),

    #[error(transparent)]
    UrlParseError(#[from] url::ParseError),

    #[error(transparent)]
    QdrantError(#[from] qdrant_client::QdrantError),

    #[error("Unable to serialize payload: {0}")]
    PayloadError(String),
}
