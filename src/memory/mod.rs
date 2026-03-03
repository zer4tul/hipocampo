//! Core memory trait and types

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Memory categories for organization
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryCategory {
    /// Long-term facts, preferences, decisions
    Core,
    /// Daily session logs
    Daily,
    /// Conversation context
    Conversation,
    /// User-defined custom category
    Custom(String),
}

impl fmt::Display for MemoryCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Core => write!(f, "core"),
            Self::Daily => write!(f, "daily"),
            Self::Conversation => write!(f, "conversation"),
            Self::Custom(name) => write!(f, "{name}"),
        }
    }
}

/// A single memory entry
#[derive(Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub key: String,
    pub content: String,
    pub category: MemoryCategory,
    pub timestamp: String,
    pub session_id: Option<String>,
    pub score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
}

impl fmt::Debug for MemoryEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MemoryEntry")
            .field("id", &self.id)
            .field("key", &self.key)
            .field("content", &self.content)
            .field("category", &self.category)
            .field("timestamp", &self.timestamp)
            .field("score", &self.score)
            .finish_non_exhaustive()
    }
}

/// Search options for recall operations
#[derive(Debug, Clone)]
pub struct SearchOptions {
    /// Maximum number of results
    pub limit: usize,
    /// Minimum relevance score
    pub min_score: Option<f64>,
    /// Filter by session
    pub session_id: Option<String>,
    /// Enable hybrid search (BM25 + Vector)
    pub hybrid: bool,
    /// Vector weight in hybrid search (0.0-1.0)
    pub vector_weight: f32,
    /// Keyword weight in hybrid search (0.0-1.0)
    pub keyword_weight: f32,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            limit: 10,
            min_score: None,
            session_id: None,
            hybrid: true,
            vector_weight: 0.7,
            keyword_weight: 0.3,
        }
    }
}

/// Filter for list operations
#[derive(Debug, Clone, Default)]
pub struct ListFilter {
    pub category: Option<MemoryCategory>,
    pub session_id: Option<String>,
    pub limit: Option<usize>,
}

/// Core memory trait — implement for any persistence backend
#[async_trait]
pub trait Memory: Send + Sync {
    /// Backend name
    fn name(&self) -> &str;

    /// Store a memory entry
    async fn store(
        &self,
        key: &str,
        content: &str,
        category: MemoryCategory,
        session_id: Option<&str>,
    ) -> crate::Result<String>;

    /// Search memories with hybrid search
    async fn search(&self, query: &str, opts: SearchOptions) -> crate::Result<Vec<MemoryEntry>>;

    /// Get a specific memory by key
    async fn get(&self, key: &str) -> crate::Result<Option<MemoryEntry>>;

    /// List memories with optional filters
    async fn list(&self, filter: ListFilter) -> crate::Result<Vec<MemoryEntry>>;

    /// Remove a memory by key
    async fn forget(&self, key: &str) -> crate::Result<bool>;

    /// Count total memories
    async fn count(&self) -> crate::Result<usize>;

    /// Health check
    async fn health_check(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_category_display_outputs_expected_values() {
        assert_eq!(MemoryCategory::Core.to_string(), "core");
        assert_eq!(MemoryCategory::Daily.to_string(), "daily");
        assert_eq!(MemoryCategory::Conversation.to_string(), "conversation");
        assert_eq!(
            MemoryCategory::Custom("project_notes".into()).to_string(),
            "project_notes"
        );
    }

    #[test]
    fn memory_entry_roundtrip_preserves_optional_fields() {
        let entry = MemoryEntry {
            id: "id-1".into(),
            key: "favorite_language".into(),
            content: "Rust".into(),
            category: MemoryCategory::Core,
            timestamp: "2026-02-16T00:00:00Z".into(),
            session_id: Some("session-abc".into()),
            score: Some(0.98),
            embedding: Some(vec![0.1, 0.2, 0.3]),
        };

        let json = serde_json::to_string(&entry).unwrap();
        let parsed: MemoryEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.id, "id-1");
        assert_eq!(parsed.key, "favorite_language");
        assert_eq!(parsed.content, "Rust");
        assert_eq!(parsed.category, MemoryCategory::Core);
        assert_eq!(parsed.session_id.as_deref(), Some("session-abc"));
        assert_eq!(parsed.score, Some(0.98));
        assert_eq!(parsed.embedding, Some(vec![0.1, 0.2, 0.3]));
    }
}
