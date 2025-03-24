use std::collections::HashMap;

use qdrant_client::{
    Qdrant,
    qdrant::{
        CreateCollectionBuilder, Distance, PointId, PointStruct, UpsertPointsBuilder, Value,
        VectorParams, VectorParamsMap, Vectors, VectorsConfig, vectors_config::Config,
    },
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::client::Storage;
use crate::{chunking::CodeChunk, embedding::Embedding, prelude::*};

pub struct QdrantStorage {
    client: Qdrant,
    collection_name: String,
    vector_name: String,
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
            vector_name: "code".to_string(),
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
            // Create the collection with named vectors
            let mut vector_params = HashMap::new();
            vector_params.insert(
                self.vector_name.clone(),
                VectorParams {
                    size: 768,
                    distance: Distance::Cosine.into(),
                    ..Default::default()
                },
            );

            self.client
                .create_collection(
                    CreateCollectionBuilder::new(self.collection_name.clone())
                        .vectors_config(VectorsConfig {
                            config: Some(Config::ParamsMap(VectorParamsMap { map: vector_params })),
                        })
                        .build(),
                )
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

        for (chunk, embedding) in chunks.iter().zip(embeddings.iter()) {
            let mut payload = HashMap::new();

            payload.insert("content".to_string(), Value::from(chunk.content.clone()));

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

            let mut vectors = HashMap::new();
            vectors.insert(self.vector_name.clone(), embedding.clone());

            let point = PointStruct {
                id: Some(PointId::from(Uuid::new_v4().to_string())),
                vectors: Some(Vectors::from(vectors)),
                payload,
            };

            points.push(point);
        }

        // Store points in batches of 100
        for batch in points.chunks(100) {
            self.client
                .upsert_points(
                    UpsertPointsBuilder::new(self.collection_name.clone(), batch.to_vec())
                        .wait(true),
                )
                .await
                .map_err(|e| Error::StorageError(e.to_string()))?;
        }

        Ok(())
    }
}
