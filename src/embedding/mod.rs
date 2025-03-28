mod client;
mod huggingface;
mod ollama;
mod openai;

pub use client::EmbeddingClient;
#[allow(unused_imports)]
pub use huggingface::HuggingFaceEmbeddingClient;
#[allow(unused_imports)]
pub use ollama::OllamaEmbeddingClient;
#[allow(unused_imports)]
pub use openai::OpenAIEmbeddingClient;

use crate::chunking::CodeChunk;
use crate::prelude::*;

pub type Embedding = Vec<f32>;

#[derive(Debug, Clone)]
pub enum EmbeddingClientImpl {
    Ollama(ollama::OllamaEmbeddingClient),
    OpenAI(openai::OpenAIEmbeddingClient),
    HuggingFace(huggingface::HuggingFaceEmbeddingClient),
}

impl EmbeddingClient for EmbeddingClientImpl {
    async fn embed(&self, chunks: &[CodeChunk]) -> Result<Vec<Embedding>> {
        match self {
            Self::Ollama(client) => client.embed(chunks).await,
            Self::OpenAI(client) => client.embed(chunks).await,
            Self::HuggingFace(client) => client.embed(chunks).await,
        }
    }

    async fn context_length(&mut self) -> Result<usize> {
        match self {
            Self::Ollama(client) => client.context_length().await,
            Self::OpenAI(client) => client.context_length().await,
            Self::HuggingFace(client) => client.context_length().await,
        }
    }

    async fn embed_length(&mut self) -> Result<usize> {
        match self {
            Self::Ollama(client) => client.embed_length().await,
            Self::OpenAI(client) => client.embed_length().await,
            Self::HuggingFace(client) => client.embed_length().await,
        }
    }
}
