use std::{
    collections::{HashMap, HashSet},
    hash::{DefaultHasher, Hash, Hasher},
};

use qdrant_client::{
    Qdrant,
    qdrant::{
        CreateCollectionBuilder, DeletePointsBuilder, Distance, PointId, PointStruct,
        PointsIdsList, ScrollPointsBuilder, UpsertPointsBuilder, Value, VectorParams,
        VectorParamsMap, Vectors, VectorsConfig, point_id::PointIdOptions,
        points_selector::PointsSelectorOneOf, vectors_config::Config,
    },
};
use serde::{Deserialize, Serialize};

use super::client::Storage;
use crate::{chunking::CodeChunk, embedding::Embedding, prelude::*};

pub struct QdrantStorage {
    client: Qdrant,
    collection_name: String,
    vector_name: String,
    embedding_size: usize,
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
    pub async fn new(url: &str, collection_name: &str, embedding_size: usize) -> Result<Self> {
        let client = Qdrant::from_url(url).skip_compatibility_check().build().map_err(Storage)?;

        let storage = Self {
            client,
            collection_name: collection_name.to_string(),
            vector_name: "code".to_string(),
            embedding_size,
        };

        // Ensure collection exists
        storage.ensure_collection().await?;

        Ok(storage)
    }

    async fn ensure_collection(&self) -> Result<()> {
        // Check if collection exists
        let collections = self.client.list_collections().await?;

        // Need the collection name to be the project root
        let exists = collections.collections.iter().any(|c| c.name == self.collection_name);

        if !exists {
            // Create the collection with named vectors
            let mut vector_params = HashMap::new();
            vector_params.insert(
                self.vector_name.clone(),
                VectorParams {
                    size: self.embedding_size as u64,
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
            return Err(Payload("Chunks and embeddings count mismatch".to_string()));
        }

        // 1. Get all existing IDs in the database
        let search_result = self
            .client
            .scroll(ScrollPointsBuilder::new(self.collection_name.clone()))
            .await
            .map_err(Storage)?;
        let mut existing_ids: HashSet<u64> = search_result
            .result
            .into_iter()
            .filter_map(|point| match point.id {
                Some(PointId {
                    point_id_options: Some(PointIdOptions::Num(n)),
                }) => Some(n),
                _ => None,
            })
            .collect();

        // 2. Batch upsert points and remove seen IDs
        let mut points_to_upsert = Vec::new();

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

            let metadata_json = serde_json::to_string(&metadata)?;

            payload.insert("metadata".to_string(), Value::from(metadata_json));

            let mut vectors = HashMap::new();
            vectors.insert(self.vector_name.clone(), embedding.clone());

            // Reproducible ID so I'm able to upsert chunks
            // TODO: Move this to the chunker trait
            let chunk_id = {
                let key = format!("{}:{}", chunk.path.display(), &chunk.node_type);
                let mut hasher = DefaultHasher::new();
                key.hash(&mut hasher);
                hasher.finish()
            };

            existing_ids.remove(&chunk_id);

            points_to_upsert.push(PointStruct::new(
                PointId::from(chunk_id),
                Vectors::from(vectors),
                payload,
            ));
        }

        for batch in points_to_upsert.chunks(100) {
            self.client
                .upsert_points(UpsertPointsBuilder::new(&self.collection_name, batch).wait(true))
                .await
                .map_err(Storage)?;
        }

        // 3. Delete remaining IDs (stale points)
        if !existing_ids.is_empty() {
            let stale_points: Vec<u64> = existing_ids.into_iter().collect();

            for batch in stale_points.chunks(100) {
                self.client
                    .delete_points(DeletePointsBuilder::new(&self.collection_name).points(
                        PointsSelectorOneOf::Points(PointsIdsList::from(batch.to_vec())),
                    ))
                    .await
                    .map_err(Storage)?;
            }
        }

        Ok(())
    }
}
