use crate::{chunking::CodeChunk, embedding::Embedding};

use crate::prelude::*;

pub trait EmbeddingClient: Send + Sync {
    async fn embed(&self, chunks: &[CodeChunk]) -> Result<Vec<Embedding>>;
    async fn context_length(&mut self) -> Result<usize>;
    async fn embed_length(&mut self) -> Result<usize>;
}
