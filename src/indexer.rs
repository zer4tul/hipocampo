//! Markdown file indexer

use crate::embedding::EmbeddingProvider;
use crate::memory::{Memory, MemoryCategory};
use crate::utils::chunker::{chunk_markdown, Chunk};
use crate::utils::hash::content_hash;
use crate::Result;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{info, warn};

/// Indexer for markdown files
pub struct MarkdownIndexer<M: Memory> {
    memory: M,
    embedder: Box<dyn EmbeddingProvider>,
    workspace: PathBuf,
}

impl<M: Memory> MarkdownIndexer<M> {
    pub fn new(memory: M, embedder: Box<dyn EmbeddingProvider>, workspace: PathBuf) -> Self {
        Self {
            memory,
            embedder,
            workspace,
        }
    }

    /// Index a single markdown file
    pub async fn index_file(&self, path: &Path, category: MemoryCategory) -> Result<usize> {
        let content = fs::read_to_string(path).await?;
        let relative_path = path.strip_prefix(&self.workspace).unwrap_or(path);

        let chunks = chunk_markdown(&content, 1000);
        info!(
            "Indexing {} -> {} chunks",
            relative_path.display(),
            chunks.len()
        );

        let mut indexed = 0;
        for chunk in chunks {
            let key = format!(
                "{}:{}:{}",
                relative_path.display(),
                chunk.start_line,
                chunk.end_line
            );

            // Check if already indexed
            if self.memory.get(&key).await?.is_some() {
                continue;
            }

            // Store memory
            self.memory
                .store(&key, &chunk.content, category.clone(), None)
                .await?;

            indexed += 1;
        }

        Ok(indexed)
    }

    /// Index all markdown files in workspace
    pub async fn index_workspace(&self) -> Result<IndexStats> {
        let mut stats = IndexStats::default();

        // Index MEMORY.md
        let memory_md = self.workspace.join("MEMORY.md");
        if memory_md.exists() {
            let count = self.index_file(&memory_md, MemoryCategory::Core).await?;
            stats.memory_md = count;
            info!("Indexed MEMORY.md: {} chunks", count);
        }

        // Index memory/*.md
        let memory_dir = self.workspace.join("memory");
        if memory_dir.exists() {
            let mut entries = fs::read_dir(&memory_dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if path.extension().map(|e| e == "md").unwrap_or(false) {
                    let count = self.index_file(&path, MemoryCategory::Daily).await?;
                    stats.daily_files += 1;
                    stats.daily_chunks += count;
                }
            }
        }

        // Count total
        stats.total = self.memory.count().await?;

        Ok(stats)
    }
}

#[derive(Debug, Default)]
pub struct IndexStats {
    pub memory_md: usize,
    pub daily_files: usize,
    pub daily_chunks: usize,
    pub total: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedding::NoopEmbedding;
    use crate::storage::sqlite::SqliteBackend;
    use tempfile::TempDir;

    #[tokio::test]
    async fn indexer_counts_chunks_correctly() {
        let temp = TempDir::new().unwrap();
        let workspace = temp.path().to_path_buf();

        // Create test file
        let memory_md = workspace.join("MEMORY.md");
        fs::write(&memory_md, "# Test\n\nContent here.\n\n## Section\n\nMore content.")
            .await
            .unwrap();

        // Setup indexer
        let embedder = Box::new(NoopEmbedding);
        let backend = SqliteBackend::new(&workspace, std::sync::Arc::new(NoopEmbedding)).unwrap();
        let indexer = MarkdownIndexer::new(backend, embedder, workspace);

        // Index
        let stats = indexer.index_workspace().await.unwrap();

        assert_eq!(stats.memory_md, 2); // 2 chunks from test file
    }
}
