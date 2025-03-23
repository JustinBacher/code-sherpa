use std::time::Duration;

use reqwest::Client;
use serde::Serialize;

use super::{Embedding, client::EmbeddingClient};
use crate::{chunking::CodeChunk, error::Error};

#[derive(Debug, Clone)]
pub struct HuggingFaceEmbeddingClient {
    client: Client,
    api_key: String,
    model: String,
}

#[derive(Serialize)]
struct HuggingFaceRequest {
    inputs: Vec<String>,
}

impl HuggingFaceEmbeddingClient {
    pub fn new(api_key: &str, model: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            api_key: api_key.to_string(),
            model: model.to_string(),
        }
    }
}

impl EmbeddingClient for HuggingFaceEmbeddingClient {
    async fn embed(&self, _chunks: &[CodeChunk]) -> Result<Vec<Embedding>, Error> {
        // Implementation for HuggingFace API
        // Similar to OpenAI but with different endpoint and request format
        todo!("Implement HuggingFace embedding client")
    }
}
