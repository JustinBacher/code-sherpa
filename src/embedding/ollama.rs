use std::collections::HashMap;

use ollama_rs::{
    Ollama,
    generation::embeddings::request::{EmbeddingsInput, GenerateEmbeddingsRequest},
};
use serde::Deserialize;
use serde_json::json;
use tracing::debug;
use url::Url;

use super::{Embedding, client::EmbeddingClient};
use crate::{chunking::CodeChunk, prelude::*};

#[derive(Debug, Clone)]
pub struct OllamaEmbeddingClient {
    client: Ollama,
    api_url: Url,
    model: String,
    batch_size: usize,
    embed_length: Option<usize>,
    context_length: Option<usize>,
}

impl OllamaEmbeddingClient {
    pub fn new(api_url: Url, port: u16, model: &str, batch_size: Option<usize>) -> Self {
        let client = Ollama::new(api_url.to_owned(), port);

        Self {
            client,
            api_url,
            model: model.to_string(),
            batch_size: batch_size.unwrap_or(512),
            embed_length: None,
            context_length: None,
        }
    }

    async fn get_model_url(&mut self) -> Result<()> {
        #[derive(Deserialize)]
        struct ModelResponse {
            model_info: HashMap<String, serde_json::Value>,
        }

        let response = reqwest::Client::new()
            .post(self.api_url.join("api/show")?)
            .json(&json!({"name": self.model}))
            .send()
            .await?;

        println!("{:?}", response);

        let response = response.json::<serde_json::Value>().await?;

        let model_response = ModelResponse::deserialize(response)
            .map_err(|e| Payload(f!("Failed to get model info. {e}")))?
            .model_info;

        self.embed_length = model_response
            .iter()
            .find(|(key, _)| key.ends_with(".embedding_length"))
            .and_then(|(_, value)| value.as_u64().map(|v| v as usize));

        self.context_length = model_response
            .iter()
            .find(|(key, _)| key.ends_with(".context_length"))
            .and_then(|(_, value)| value.as_u64().map(|v| v as usize));

        Ok(())
    }
}

impl EmbeddingClient for OllamaEmbeddingClient {
    async fn embed(&self, chunks: &[CodeChunk]) -> Result<Vec<Embedding>> {
        let mut all_embeddings = Vec::with_capacity(chunks.len());

        for chunk_batch in chunks.chunks(self.batch_size) {
            let mut batch_embeddings = Vec::with_capacity(chunk_batch.len());

            for chunk in chunk_batch {
                debug!("Generating embedding for chunk from {:?}", chunk.path);

                let request = GenerateEmbeddingsRequest::new(
                    self.model.to_string(),
                    EmbeddingsInput::Single(chunk.content.to_string()),
                );
                let response = self.client.generate_embeddings(request).await?;

                batch_embeddings.extend(response.embeddings);
            }

            all_embeddings.extend(batch_embeddings);
        }

        debug!("Generated {} embeddings with Ollama", all_embeddings.len());
        Ok(all_embeddings)
    }

    async fn context_length(&mut self) -> Result<usize> {
        if self.context_length.is_none() {
            self.get_model_url().await?;
        }

        self.context_length.ok_or(Missing(String::from("Context length not found")))
    }

    async fn embed_length(&mut self) -> Result<usize> {
        if self.context_length.is_none() {
            self.get_model_url().await?;
        }

        self.embed_length.ok_or(Missing(String::from("Embedding length not found")))
    }
}
