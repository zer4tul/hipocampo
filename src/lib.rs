//! Hipocampo - Agent-first unified memory backend
//!
//! A memory backend compatible with both OpenClaw (TypeScript) and ZeroClaw (Rust),
//! using Markdown as the source of truth and SQLite + vectors as derived indices.

pub mod memory;
pub mod storage;
pub mod embedding;
pub mod search;
pub mod utils;
pub mod indexer;

pub use memory::{Memory, MemoryCategory, MemoryEntry, SearchOptions, ListFilter};
pub use storage::sqlite::SqliteBackend;
pub use embedding::EmbeddingProvider;
pub use indexer::{MarkdownIndexer, IndexStats};

pub type Result<T> = anyhow::Result<T>;
