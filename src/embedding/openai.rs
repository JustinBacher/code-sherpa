use std::time::Duration;

use reqwest::Client as ReqwestClient;
use serde::{Deserialize, Serialize};

use super::{Embedding, client::EmbeddingClient};
use crate::{chunking::CodeChunk, error::Error, prelude::*};

#[derive(Debug, Clone)]
pub struct OpenAIEmbeddingClient {
    client: ReqwestClient,
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

const OPENAI_API_URL: &str = "https://api.openai.com/v1/embeddings";

impl OpenAIEmbeddingClient {
    pub fn new(api_key: &str, model: &str) -> Self {
        let client = ReqwestClient::builder()
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
        // FIXME: This is AI generated, I don't have an API key so need to find out if this works
        // at some point

        let texts: Vec<String> = chunks.iter().map(|chunk| chunk.content.clone()).collect();

        let mut all_embeddings = Vec::new();

        for batch in texts.chunks(20) {
            let request = OpenAIEmbeddingRequest {
                model: self.model.clone(),
                input: batch.to_vec(),
            };

            let response = self
                .client
                .post(OPENAI_API_URL)
                .header("Authorization", f!("Bearer {}", self.api_key))
                .json(&request)
                .send()
                .await?;

            if !response.status().is_success() {
                let error_text = response.text().await?;
                return Err(Error::Embedding(error_text));
            }

            let embedding_response: OpenAIEmbeddingResponse = response.json().await?;

            all_embeddings.extend(embedding_response.data.into_iter().map(|data| data.embedding));
        }

        Ok(all_embeddings)
    }

    async fn context_length(&mut self) -> Result<usize> {
        // FIXME: This is AI generated, I don't have an API key so need to find out if this works
        // at some point

        Ok(match self.model.as_str() {
            "text-embedding-ada-002" => 8191,
            "text-embedding-3-small" => 8191,
            "text-embedding-3-large" => 8191,
            _ => 2048,
        })
    }

    async fn embed_length(&mut self) -> Result<usize> {
        // FIXME: This is AI generated, I don't have an API key so need to find out if this works
        // at some point

        Ok(match self.model.as_str() {
            "text-embedding-ada-002" => 1536,
            "text-embedding-3-small" => 1536,
            "text-embedding-3-large" => 3072,
            _ => {
                // For unknown models, make a small test request
                let test_response = self
                    .client
                    .post(OPENAI_API_URL)
                    .header("Authorization", format!("Bearer {}", self.api_key))
                    .json(&OpenAIEmbeddingRequest {
                        model: self.model.clone(),
                        input: vec!["test".to_string()],
                    })
                    .send()
                    .await?;

                let embedding_response: OpenAIEmbeddingResponse = test_response.json().await?;
                if embedding_response.data.is_empty() {
                    return Err(Error::Embedding("Empty embedding response".to_string()));
                }

                embedding_response.data[0].embedding.len()
            },
        })
    }
}
