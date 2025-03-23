use crate::{chunking::CodeChunk, embedding::Embedding, error::Error};

pub trait Storage {
    async fn store_chunks(
        &self,
        chunks: &[CodeChunk],
        embeddings: &[Embedding],
    ) -> Result<(), Error>;
}
