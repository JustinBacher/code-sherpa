use std::{fs, path::Path};

use tracing::{info, warn};
use tree_sitter::Parser;
use walkdir::{DirEntry, WalkDir};

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
    pub overlap_percentage: Option<usize>,
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
        Self {
            parser: Parser::new(),
            embedding_client,
            storage,
            config,
        }
    }

    pub async fn scan_codebase(&mut self, root: &Path) -> Result<ScanResults> {
        let mut chunks = Vec::new();

        for entry in WalkDir::new(root)
            .into_iter()
            .filter_entry(is_wanted_directory)
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            if let Some(extension) = path.extension() {
                if let Ok(parser) =
                    serde_plain::from_str::<SupportedParsers>(&extension.to_string_lossy())
                {
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

        // Generate embeddings
        let embeddings = self.embedding_client.embed(&chunks).await?;

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

        let chunks = extract_chunks(
            &tree,
            content,
            path,
            language,
            self.config.chunk_size_limit,
            self.config.overlap_percentage,
        );
        info!("Extracted {} chunks from {path:?}", chunks.len());
        Ok(chunks)
    }
}

fn is_wanted_directory(entry: &DirEntry) -> bool {
    if !entry.path().is_dir() {
        return true; // Always include files
    }

    entry
        .file_name()
        .to_str()
        .map(|s| s != "target" && s != ".git" && s != "node_modules")
        .unwrap_or(false)
}
