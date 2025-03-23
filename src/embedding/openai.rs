use std::time::Duration;

use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{Embedding, client::EmbeddingClient};
use crate::{chunking::CodeChunk, error::Error, prelude::*};

#[derive(Debug, Clone)]
pub struct OpenAIEmbeddingClient {
    client: Client,
    api_key: String,
    model: String,
}

#[derive(Serialize)]
struct OpenAIEmbeddingRequest {
    model: String,
    input: Vec<String>,
}

#[derive(Deserialize)]
struct OpenAIEmbeddingResponse {
    data: Vec<OpenAIEmbeddingData>,
}

#[derive(Deserialize)]
struct OpenAIEmbeddingData {
    embedding: Vec<f32>,
}

impl OpenAIEmbeddingClient {
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

impl EmbeddingClient for OpenAIEmbeddingClient {
    async fn embed(&self, chunks: &[CodeChunk]) -> Result<Vec<Embedding>> {
        // Extract the content from chunks
        let texts: Vec<String> = chunks.iter().map(|chunk| chunk.content.clone()).collect();

        // Process in batches of 20
        let mut all_embeddings = Vec::new();

        for batch in texts.chunks(20) {
            let request = OpenAIEmbeddingRequest {
                model: self.model.clone(),
                input: batch.to_vec(),
            };

            let response = self
                .client
                .post("https://api.openai.com/v1/embeddings")
                .header("Authorization", f!("Bearer {}", self.api_key))
                .json(&request)
                .send()
                .await?;

            if !response.status().is_success() {
                let error_text = response.text().await?;
                return Err(Error::EmbeddingError(error_text));
            }

            let embedding_response: OpenAIEmbeddingResponse = response.json().await?;

            all_embeddings.extend(embedding_response.data.into_iter().map(|data| data.embedding));
        }

        Ok(all_embeddings)
    }
}
