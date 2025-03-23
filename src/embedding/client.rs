use crate::{chunking::CodeChunk, embedding::Embedding, error::Error};

pub trait EmbeddingClient: Send + Sync {
    async fn embed(&self, chunks: &[CodeChunk]) -> Result<Vec<Embedding>, Error>;
}
