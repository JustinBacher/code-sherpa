use std::{fs, path::Path, str::FromStr};

use tracing::{info, warn};
use tree_sitter::Parser;
use walkdir::WalkDir;

use super::results::ScanResults;
use crate::{
    chunking::{CodeChunk, extract_chunks},
    embedding::EmbeddingClient,
    prelude::*,
    storage::Storage,
    utils::parsers::SupportedParsers,
};

pub struct ScannerConfig {
    pub chunk_size_limit: Option<usize>,
}

pub struct CodebaseScanner<E, S>
where
    E: EmbeddingClient,
    S: Storage,
{
    parser: Parser,
    embedding_client: E,
    storage: S,
    config: ScannerConfig,
}

impl<E, S> CodebaseScanner<E, S>
where
    E: EmbeddingClient,
    S: Storage,
{
    pub fn new(embedding_client: E, storage: S, config: ScannerConfig) -> Self {
        let parser = Parser::new();

        Self {
            parser,
            embedding_client,
            storage,
            config,
        }
    }

    pub async fn scan_codebase(&mut self, root: &Path) -> Result<ScanResults> {
        let mut chunks = Vec::new();

        for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            if let Some(extension) = path.extension() {
                if let Ok(parser) = SupportedParsers::from_str(&extension.to_string_lossy()) {
                    match fs::read_to_string(path) {
                        Ok(content) => match self.parse_file(path, &content, &parser) {
                            Ok(file_chunks) => chunks.extend(file_chunks),
                            Err(e) => warn!("Failed to parse {}: {}", path.display(), e),
                        },
                        Err(e) => warn!("Failed to read {}: {}", path.display(), e),
                    }
                }
            }
        }

        info!("Extracted {} chunks from codebase", chunks.len());

        // Generate embeddings
        let embeddings = self.embedding_client.embed(&chunks).await?;

        info!("Generated {} embeddings", embeddings.len());

        // Store the embeddings
        self.storage.store_chunks(&chunks, &embeddings).await?;

        Ok(ScanResults {
            chunks_processed: chunks.len(),
            embeddings_generated: embeddings.len(),
        })
    }

    fn parse_file(
        &mut self,
        path: &Path,
        content: &str,
        language: &SupportedParsers,
    ) -> Result<Vec<CodeChunk>> {
        self.parser.set_language(&language.language())?;

        let tree = self.parser.parse(content, None).ok_or(ParsingFailed(path.to_path_buf()))?;

        Ok(extract_chunks(&tree, content, path, language))
    }
}
