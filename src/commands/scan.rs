use std::{env, path::PathBuf, str::FromStr};

use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use tracing::{error, info};
use url::Url;

use super::Command;
use crate::{
    embedding::{
        EmbeddingClientImpl, HuggingFaceEmbeddingClient, OllamaEmbeddingClient,
        OpenAIEmbeddingClient,
    },
    prelude::*,
    scanner::{CodebaseScanner, ScannerConfig},
    storage::QdrantStorage,
    utils::path_to_collection_name,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Address {
    pub url: Url,
    pub port: Option<u16>,
}

impl FromStr for Address {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let url = Url::parse(s).map_err(|e| InvalidArgument(f!("Unable to parse address {e}")))?;
        let port = url.port();

        Ok(Self { url, port })
    }
}

#[derive(Debug, Clone, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ClientType {
    Ollama,
    OpenAI,
    HuggingFace,
}

#[derive(Debug, Parser, Serialize, Deserialize, Clone)]
pub struct Scan {
    #[arg(long, value_enum)]
    client: ClientType,

    // Ollama-specific args
    #[arg(long, required_if_eq("client", "Ollama"))]
    address: Option<Address>,

    #[arg(long, short)]
    model: Option<String>,

    /// Qdrant URL
    #[arg(long, default_value = "http://localhost:6334")]
    qdrant_url: String,

    /// Collection name for storage
    #[arg(long, default_value = "code-sherpa")]
    collection: String,

    /// Filter by file extensions (comma-separated)
    #[arg(short, long)]
    extensions: Option<String>,

    /// Chunk size limit (in bytes)
    #[arg(short, long)]
    chunk_size_limit: Option<usize>,

    /// Percentage of overlap between chunks (default: 10%)
    #[arg(long, default_value = "10")]
    overlap_percentage: Option<usize>,

    /// Path to the codebase root
    #[arg(short, long)]
    path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ClientConfig {
    Ollama { address: Address, model: String },
    OpenAI { api_key: String, model: String },
    HuggingFace { api_key: String, model: String },
}

impl Command for Scan {
    async fn execute(&self) -> Result<()> {
        if !self.path.exists() {
            error!("Path does not exist: {}", self.path.display());
            return Err(NotFound(self.path.clone()));
        }

        let model = self.model.clone().unwrap_or(
            match self.client {
                ClientType::Ollama => "nomic-embed-text",
                ClientType::OpenAI => "gpt-4o",
                ClientType::HuggingFace => "snowflake-arctic-embed-l-v2.0",
            }
            .to_string(),
        );

        let api_key = match self.client {
            ClientType::Ollama => Ok(String::from("")),
            ClientType::OpenAI => env::var("OPENAI_API_KEY"),
            ClientType::HuggingFace => env::var("HUGGINGFACE_API_KEY"),
        }
        .map_err(|_| Missing(String::from("API key environment variable not set")))?;

        info!("Scanning codebase at {}", self.path.display());
        info!("Using embedding model: {}", model);

        // Parse extensions filter if provided
        let extensions = self
            .extensions
            .clone()
            .map(|ext_str| ext_str.split(',').map(|s| s.trim().to_string()).collect::<Vec<_>>());

        if let Some(ref exts) = extensions {
            info!("Filtering by extensions: {}", exts.join(", "));
        }

        if let Some(chunk_size) = self.chunk_size_limit {
            info!("Using chunk size limit: {} bytes", chunk_size);
        }

        info!(
            "Using chunk overlap: {}%",
            self.overlap_percentage.unwrap_or(10)
        );

        let embedding_client = match self.client {
            ClientType::Ollama => {
                let address = self.address.clone().unwrap_or_else(|| {
                    Address::from_str("http://localhost:11434")
                        .expect("Default address should be valid")
                });
                EmbeddingClientImpl::Ollama(OllamaEmbeddingClient::new(
                    address.url.as_str(),
                    address.port.unwrap_or(11434),
                    &model,
                    self.chunk_size_limit,
                ))
            },
            ClientType::OpenAI => {
                EmbeddingClientImpl::OpenAI(OpenAIEmbeddingClient::new(&api_key, &model))
            },
            ClientType::HuggingFace => {
                EmbeddingClientImpl::HuggingFace(HuggingFaceEmbeddingClient::new(&api_key, &model))
            },
        };

        let storage =
            QdrantStorage::new(&self.qdrant_url, &path_to_collection_name(&self.path)).await?;

        info!("Starting codebase scan");
        let scanner_config = ScannerConfig {
            chunk_size_limit: self.chunk_size_limit,
            overlap_percentage: self.overlap_percentage,
        };

        let mut scanner = CodebaseScanner::new(embedding_client, storage, scanner_config);

        match scanner.scan_codebase(&self.path).await {
            Ok(results) => {
                info!("Scan completed successfully");
                info!("Processed {} code chunks", results.chunks_processed);
                info!("Generated {} embeddings", results.embeddings_generated);
                info!("Stored in collection: {}", self.collection);
                Ok(())
            },
            Err(e) => {
                error!("Scan failed: {}", e);
                Err(ScanFailed)
            },
        }
    }
}
