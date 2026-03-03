//! SQLite storage backend

use crate::embedding::EmbeddingProvider;
use crate::memory::{ListFilter, Memory, MemoryCategory, MemoryEntry, SearchOptions};
use crate::Result;
use async_trait::async_trait;
use chrono::Utc;
use rusqlite::params;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

/// SQLite-backed memory backend
pub struct SqliteBackend {
    db_path: PathBuf,
    conn: Arc<Mutex<rusqlite::Connection>>,
    embedder: Arc<dyn EmbeddingProvider>,
    vector_weight: f32,
    keyword_weight: f32,
}

impl SqliteBackend {
    /// Create a new SQLite backend
    pub fn new(workspace_dir: &std::path::Path, embedder: Arc<dyn EmbeddingProvider>) -> Result<Self> {
        let db_path = workspace_dir.join("memory").join("hipocampo.db");

        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Self::open_connection(&db_path)?;
        Self::init_schema(&conn)?;

        Ok(Self {
            db_path,
            conn: Arc::new(Mutex::new(conn)),
            embedder,
            vector_weight: 0.7,
            keyword_weight: 0.3,
        })
    }

    fn open_connection(db_path: &std::path::Path) -> Result<rusqlite::Connection> {
        let conn = rusqlite::Connection::open(db_path)?;

        // Performance optimizations
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA mmap_size = 8388608;
             PRAGMA cache_size = -2000;
             PRAGMA temp_store = MEMORY;",
        )?;

        Ok(conn)
    }

    fn init_schema(conn: &rusqlite::Connection) -> Result<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS memories (
                id          TEXT PRIMARY KEY,
                key         TEXT NOT NULL UNIQUE,
                content     TEXT NOT NULL,
                category    TEXT NOT NULL DEFAULT 'core',
                embedding   BLOB,
                created_at  TEXT NOT NULL,
                updated_at  TEXT NOT NULL,
                session_id  TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_memories_category ON memories(category);
            CREATE INDEX IF NOT EXISTS idx_memories_key ON memories(key);
            CREATE INDEX IF NOT EXISTS idx_memories_session ON memories(session_id);

            CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
                key, content, content=memories, content_rowid=rowid
            );

            CREATE TRIGGER IF NOT EXISTS memories_ai AFTER INSERT ON memories BEGIN
                INSERT INTO memories_fts(rowid, key, content)
                VALUES (new.rowid, new.key, new.content);
            END;

            CREATE TRIGGER IF NOT EXISTS memories_ad AFTER DELETE ON memories BEGIN
                INSERT INTO memories_fts(memories_fts, rowid, key, content)
                VALUES('delete', old.rowid, old.key, old.content);
            END;

            CREATE TRIGGER IF NOT EXISTS memories_au AFTER UPDATE ON memories BEGIN
                INSERT INTO memories_fts(memories_fts, rowid, key, content)
                VALUES('delete', old.rowid, old.key, old.content);
                INSERT INTO memories_fts(rowid, key, content)
                VALUES (new.rowid, new.key, new.content);
            END;",
        )?;

        Ok(())
    }

    async fn embed_and_store(&self, id: &str, content: &str) -> Result<()> {
        let embedding = self.embedder.embed_one(content).await.ok();

        let conn = self.conn.lock().await;
        if let Some(vec) = embedding {
            let blob = Self::vec_to_blob(&vec);
            conn.execute(
                "UPDATE memories SET embedding = ?1 WHERE id = ?2",
                params![blob, id],
            )?;
        }

        Ok(())
    }

    fn vec_to_blob(vec: &[f32]) -> Vec<u8> {
        let mut blob = Vec::with_capacity(vec.len() * 4);
        for f in vec {
            blob.extend_from_slice(&f.to_le_bytes());
        }
        blob
    }

    fn blob_to_vec(blob: &[u8]) -> Vec<f32> {
        blob.chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect()
    }

    async fn search_vector(&self, query_embedding: &[f32], limit: usize, session_id: Option<&str>) -> Result<Vec<MemoryEntry>> {
        let conn = self.conn.lock().await;

        let sql = if session_id.is_some() {
            "SELECT id, key, content, category, created_at, session_id, embedding
             FROM memories
             WHERE embedding IS NOT NULL AND session_id = ?1
             ORDER BY vector_cosine_similarity(embedding, ?2) DESC
             LIMIT ?3"
        } else {
            "SELECT id, key, content, category, created_at, session_id, embedding
             FROM memories
             WHERE embedding IS NOT NULL
             ORDER BY vector_cosine_similarity(embedding, ?1) DESC
             LIMIT ?2"
        };

        let query_blob = Self::vec_to_blob(query_embedding);
        let mut stmt = conn.prepare_cached(sql)?;

        let results = match session_id {
            Some(sid) => {
                let rows = stmt.query_map(params![sid, query_blob, limit], Self::map_vector_row(query_embedding))?;
                Self::collect_rows(rows)?
            }
            None => {
                let rows = stmt.query_map(params![query_blob, limit], Self::map_vector_row(query_embedding))?;
                Self::collect_rows(rows)?
            }
        };

        Ok(results)
    }

    fn map_vector_row(query_embedding: &[f32]) -> impl Fn(&rusqlite::Row) -> rusqlite::Result<MemoryEntry> + '_ {
        move |row| {
            Ok(MemoryEntry {
                id: row.get(0)?,
                key: row.get(1)?,
                content: row.get(2)?,
                category: Self::parse_category(&row.get::<_, String>(3)?),
                timestamp: row.get(4)?,
                session_id: row.get(5)?,
                score: Some(Self::cosine_similarity(query_embedding, &Self::blob_to_vec(&row.get::<_, Vec<u8>>(6)?))),
                embedding: None,
            })
        }
    }

    fn collect_rows(rows: rusqlite::MappedRows<impl FnMut(&rusqlite::Row) -> rusqlite::Result<MemoryEntry>>) -> Result<Vec<MemoryEntry>> {
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    fn collect_rows_fts(rows: rusqlite::MappedRows<impl FnMut(&rusqlite::Row) -> rusqlite::Result<MemoryEntry>>) -> Result<Vec<MemoryEntry>> {
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    async fn search_fts(&self, query: &str, limit: usize, session_id: Option<&str>) -> Result<Vec<MemoryEntry>> {
        let conn = self.conn.lock().await;

        let fts_query = Self::build_fts_query(query);
        let sql = if session_id.is_some() {
            "SELECT m.id, m.key, m.content, m.category, m.created_at, m.session_id, fts.rank
             FROM memories m
             JOIN memories_fts fts ON m.rowid = fts.rowid
             WHERE memories_fts MATCH ?1 AND m.session_id = ?2
             ORDER BY bm25(memories_fts) ASC
             LIMIT ?3"
        } else {
            "SELECT m.id, m.key, m.content, m.category, m.created_at, m.session_id, fts.rank
             FROM memories m
             JOIN memories_fts fts ON m.rowid = fts.rowid
             WHERE memories_fts MATCH ?1
             ORDER BY bm25(memories_fts) ASC
             LIMIT ?2"
        };

        let mut stmt = conn.prepare_cached(sql)?;

        let results = match session_id {
            Some(sid) => {
                let rows = stmt.query_map(params![&fts_query, sid, limit], Self::map_fts_row())?;
                Self::collect_rows_fts(rows)?
            }
            None => {
                let rows = stmt.query_map(params![&fts_query, limit], Self::map_fts_row())?;
                Self::collect_rows_fts(rows)?
            }
        };

        Ok(results)
    }

    fn map_fts_row() -> impl Fn(&rusqlite::Row) -> rusqlite::Result<MemoryEntry> {
        |row| {
            let rank: f64 = row.get(6)?;
            Ok(MemoryEntry {
                id: row.get(0)?,
                key: row.get(1)?,
                content: row.get(2)?,
                category: Self::parse_category(&row.get::<_, String>(3)?),
                timestamp: row.get(4)?,
                session_id: row.get(5)?,
                score: Some(Self::bm25_rank_to_score(rank)),
                embedding: None,
            })
        }
    }

    fn build_fts_query(raw: &str) -> String {
        let tokens: Vec<&str> = raw
            .split_whitespace()
            .filter(|t| !t.is_empty())
            .collect();
        if tokens.is_empty() {
            return "\"\"".to_string();
        }
        tokens
            .iter()
            .map(|t| format!("\"{}\"", t.replace('"', "")))
            .collect::<Vec<_>>()
            .join(" AND ")
    }

    fn bm25_rank_to_score(rank: f64) -> f64 {
        let normalized = if rank.is_finite() && rank >= 0.0 { rank } else { 999.0 };
        1.0 / (1.0 + normalized)
    }

    fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }

        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if mag_a == 0.0 || mag_b == 0.0 {
            return 0.0;
        }

        (dot / (mag_a * mag_b)) as f64
    }

    fn parse_category(s: &str) -> MemoryCategory {
        match s {
            "core" => MemoryCategory::Core,
            "daily" => MemoryCategory::Daily,
            "conversation" => MemoryCategory::Conversation,
            other => MemoryCategory::Custom(other.to_string()),
        }
    }
}

#[async_trait]
impl Memory for SqliteBackend {
    fn name(&self) -> &str {
        "sqlite"
    }

    async fn store(
        &self,
        key: &str,
        content: &str,
        category: MemoryCategory,
        session_id: Option<&str>,
    ) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let category_str = category.to_string();

        {
            let conn = self.conn.lock().await;
            conn.execute(
                "INSERT INTO memories (id, key, content, category, created_at, updated_at, session_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![id, key, content, category_str, now, now, session_id],
            )?;
        }

        // Async embedding (non-blocking)
        self.embed_and_store(&id, content).await?;

        Ok(id)
    }

    async fn search(&self, query: &str, opts: SearchOptions) -> Result<Vec<MemoryEntry>> {
        if !opts.hybrid {
            // Keyword-only search
            return self.search_fts(query, opts.limit, opts.session_id.as_deref()).await;
        }

        // Hybrid search
        let limit = opts.limit;
        let query_embedding = self.embedder.embed_one(query).await?;

        let (vector_results, keyword_results) = tokio::try_join!(
            self.search_vector(&query_embedding, limit * 2, opts.session_id.as_deref()),
            self.search_fts(query, limit * 2, opts.session_id.as_deref())
        )?;

        // Merge results
        let mut by_id = std::collections::HashMap::new();

        for r in vector_results {
            by_id.insert(
                r.id.clone(),
                MemoryEntry {
                    id: r.id,
                    key: r.key,
                    content: r.content,
                    category: r.category,
                    timestamp: r.timestamp,
                    session_id: r.session_id,
                    score: Some(r.score.unwrap_or(0.0) * self.vector_weight as f64),
                    embedding: None,
                },
            );
        }

        for r in keyword_results {
            let entry = by_id.entry(r.id.clone()).or_insert(MemoryEntry {
                id: r.id,
                key: r.key,
                content: r.content,
                category: r.category,
                timestamp: r.timestamp,
                session_id: r.session_id,
                score: Some(0.0),
                embedding: None,
            });
            entry.score = Some(
                entry.score.unwrap_or(0.0) + r.score.unwrap_or(0.0) * self.keyword_weight as f64,
            );
        }

        // Sort and limit
        let mut merged: Vec<_> = by_id.into_values().collect();
        merged.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        merged.truncate(limit);

        Ok(merged)
    }

    async fn get(&self, key: &str) -> Result<Option<MemoryEntry>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn.prepare_cached(
            "SELECT id, key, content, category, created_at, session_id
             FROM memories WHERE key = ?1",
        )?;

        let mut rows = stmt.query_map(params![key], |row| {
            Ok(MemoryEntry {
                id: row.get(0)?,
                key: row.get(1)?,
                content: row.get(2)?,
                category: Self::parse_category(&row.get::<_, String>(3)?),
                timestamp: row.get(4)?,
                session_id: row.get(5)?,
                score: None,
                embedding: None,
            })
        })?;

        Ok(rows.next().transpose()?)
    }

    async fn list(&self, filter: ListFilter) -> Result<Vec<MemoryEntry>> {
        let conn = self.conn.lock().await;

        let mut sql = "SELECT id, key, content, category, created_at, session_id FROM memories".to_string();
        let mut conditions = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref cat) = filter.category {
            conditions.push("category = ?");
            params_vec.push(Box::new(cat.to_string()));
        }

        if let Some(ref sid) = filter.session_id {
            conditions.push("session_id = ?");
            params_vec.push(Box::new(sid.clone()));
        }

        if !conditions.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&conditions.join(" AND "));
        }

        sql.push_str(" ORDER BY created_at DESC");

        if let Some(limit) = filter.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        let mut stmt = conn.prepare_cached(&sql)?;

        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

        let rows = stmt.query_map(params_refs.as_slice(), |row| {
            Ok(MemoryEntry {
                id: row.get(0)?,
                key: row.get(1)?,
                content: row.get(2)?,
                category: Self::parse_category(&row.get::<_, String>(3)?),
                timestamp: row.get(4)?,
                session_id: row.get(5)?,
                score: None,
                embedding: None,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }

        Ok(results)
    }

    async fn forget(&self, key: &str) -> Result<bool> {
        let conn = self.conn.lock().await;
        let affected = conn.execute("DELETE FROM memories WHERE key = ?1", params![key])?;
        Ok(affected > 0)
    }

    async fn count(&self) -> Result<usize> {
        let conn = self.conn.lock().await;
        let count: usize = conn.query_row("SELECT COUNT(*) FROM memories", [], |row| row.get(0))?;
        Ok(count)
    }

    async fn health_check(&self) -> bool {
        let conn = self.conn.lock().await;
        conn.execute("SELECT 1", []).is_ok()
    }
}
