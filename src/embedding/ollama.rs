use ollama_rs::{
    Ollama,
    generation::embeddings::request::{EmbeddingsInput, GenerateEmbeddingsRequest},
};
use tracing::debug;

use super::{Embedding, client::EmbeddingClient};
use crate::{chunking::CodeChunk, prelude::*};

#[derive(Debug, Clone)]
pub struct OllamaEmbeddingClient {
    client: Ollama,
    api_url: String,
    model: String,
    batch_size: usize,
}

impl OllamaEmbeddingClient {
    pub fn new(api_url: &str, port: u16, model: &str, batch_size: Option<usize>) -> Self {
        let client = Ollama::new(api_url, port);

        Self {
            client,
            api_url: f!("{}/api/embeddings", api_url.trim_end_matches('/')),
            model: model.to_string(),
            batch_size: batch_size.unwrap_or(512),
        }
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
}
