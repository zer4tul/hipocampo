//! ZeroClaw adapter - implements ZeroClaw's Memory trait

use crate::memory::{Memory as HipocampoMemory, MemoryCategory, MemoryEntry, SearchOptions};
use crate::storage::sqlite::SqliteBackend;
use crate::embedding::NoopEmbedding;
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;

/// ZeroClaw-compatible Memory trait
#[async_trait]
pub trait ZeroClawMemory: Send + Sync {
    /// Backend name
    fn name(&self) -> &str;

    /// Store a memory entry
    async fn store(
        &self,
        key: &str,
        content: &str,
        category: MemoryCategory,
        session_id: Option<&str>,
    ) -> crate::Result<()>;

    /// Recall memories matching a query
    async fn recall(
        &self,
        query: &str,
        limit: usize,
        session_id: Option<&str>,
    ) -> crate::Result<Vec<MemoryEntry>>;

    /// Get a specific memory by key
    async fn get(&self, key: &str) -> crate::Result<Option<MemoryEntry>>;

    /// List all memory keys
    async fn list(
        &self,
        category: Option<&MemoryCategory>,
        session_id: Option<&str>,
    ) -> crate::Result<Vec<MemoryEntry>>;

    /// Remove a memory by key
    async fn forget(&self, key: &str) -> crate::Result<bool>;

    /// Count total memories
    async fn count(&self) -> crate::Result<usize>;

    /// Health check
    async fn health_check(&self) -> bool;
}

/// ZeroClaw adapter wrapping Hipocampo's SQLite backend
pub struct ZeroClawAdapter {
    backend: SqliteBackend,
}

impl ZeroClawAdapter {
    pub fn new(workspace: PathBuf) -> crate::Result<Self> {
        let embedder = Arc::new(NoopEmbedding);
        let backend = SqliteBackend::new(&workspace, embedder)?;

        Ok(Self { backend })
    }
}

#[async_trait]
impl ZeroClawMemory for ZeroClawAdapter {
    fn name(&self) -> &str {
        "hipocampo-zeroclaw-adapter"
    }

    async fn store(
        &self,
        key: &str,
        content: &str,
        category: MemoryCategory,
        session_id: Option<&str>,
    ) -> crate::Result<()> {
        self.backend.store(key, content, category, session_id).await?;
        Ok(())
    }

    async fn recall(
        &self,
        query: &str,
        limit: usize,
        session_id: Option<&str>,
    ) -> crate::Result<Vec<MemoryEntry>> {
        let opts = SearchOptions {
            limit,
            session_id: session_id.map(|s| s.to_string()),
            hybrid: false,
            ..Default::default()
        };

        self.backend.search(query, opts).await
    }

    async fn get(&self, key: &str) -> crate::Result<Option<MemoryEntry>> {
        self.backend.get(key).await
    }

    async fn list(
        &self,
        category: Option<&MemoryCategory>,
        session_id: Option<&str>,
    ) -> crate::Result<Vec<MemoryEntry>> {
        let filter = crate::memory::ListFilter {
            category: category.cloned(),
            session_id: session_id.map(|s| s.to_string()),
            ..Default::default()
        };

        self.backend.list(filter).await
    }

    async fn forget(&self, key: &str) -> crate::Result<bool> {
        self.backend.forget(key).await
    }

    async fn count(&self) -> crate::Result<usize> {
        self.backend.count().await
    }

    async fn health_check(&self) -> bool {
        self.backend.health_check().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn adapter_stores_and_recalls() {
        let temp = TempDir::new().unwrap();
        let adapter = ZeroClawAdapter::new(temp.path().to_path_buf()).unwrap();

        adapter
            .store("test-key", "Test content", MemoryCategory::Core, None)
            .await
            .unwrap();

        let entry = adapter.get("test-key").await.unwrap();
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().content, "Test content");

        let results = adapter.recall("Test", 10, None).await.unwrap();
        assert!(!results.is_empty());
    }
}
