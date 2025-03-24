use std::collections::HashMap;

use qdrant_client::{
    Qdrant,
    qdrant::{CreateCollection, PointStruct, UpsertPoints, Value},
};
use serde::{Deserialize, Serialize};

use super::client::Storage;
use crate::{chunking::CodeChunk, embedding::Embedding, prelude::*};

pub struct QdrantStorage {
    client: Qdrant,
    collection_name: String,
}

#[derive(Serialize, Deserialize)]
struct ChunkMetadata {
    path: String,
    node_type: String,
    start_line: usize,
    end_line: usize,
    language: String,
}

impl QdrantStorage {
    pub async fn new(url: &str, collection_name: &str) -> Result<Self> {
        let client = Qdrant::from_url(url)
            .skip_compatibility_check()
            .build()
            .map_err(|e| Error::StorageError(e.to_string()))?;

        let storage = Self {
            client,
            collection_name: collection_name.to_string(),
        };

        // Ensure collection exists
        storage.ensure_collection().await?;

        Ok(storage)
    }

    async fn ensure_collection(&self) -> Result<()> {
        // Check if collection exists
        let collections = self
            .client
            .list_collections()
            .await
            .map_err(|e| Error::StorageError(e.to_string()))?;

        // Need the collection name to be the project root
        let exists = collections.collections.iter().any(|c| c.name == self.collection_name);

        if !exists {
            // Create the collection
            self.client
                .create_collection(CreateCollection {
                    collection_name: self.collection_name.clone(),
                    hnsw_config: None,
                    vectors_config: None,
                    ..Default::default()
                })
                .await?;
        }

        Ok(())
    }
}

impl Storage for QdrantStorage {
    async fn store_chunks(&self, chunks: &[CodeChunk], embeddings: &[Embedding]) -> Result<()> {
        if chunks.len() != embeddings.len() {
            return Err(Error::StorageError(
                "Chunks and embeddings count mismatch".to_string(),
            ));
        }

        let mut points = Vec::with_capacity(chunks.len());

        for (i, (chunk, embedding)) in chunks.iter().zip(embeddings.iter()).enumerate() {
            // Create metadata
            let mut payload = HashMap::new();

            // Add code content
            payload.insert("content".to_string(), Value::from(chunk.content.clone()));

            // Add metadata
            let metadata = ChunkMetadata {
                path: chunk.path.to_string_lossy().to_string(),
                node_type: chunk.node_type.clone(),
                start_line: chunk.start_line,
                end_line: chunk.end_line,
                language: chunk.language.clone(),
            };

            let metadata_json =
                serde_json::to_string(&metadata).map_err(|e| StorageError(e.to_string()))?;

            payload.insert("metadata".to_string(), Value::from(metadata_json));

            // Create point
            let point = PointStruct {
                id: Some(i.to_string().into()),
                vectors: Some(embedding.clone().into()),
                payload,
            };

            points.push(point);
        }

        // Store points in batches of 100
        for batch in points.chunks(100) {
            self.client
                .upsert_points(UpsertPoints {
                    collection_name: self.collection_name.clone(),
                    wait: None,
                    points: batch.to_vec(),
                    ordering: None,
                    shard_key_selector: None,
                })
                .await
                .map_err(|e| Error::StorageError(e.to_string()))?;
        }

        Ok(())
    }
}
