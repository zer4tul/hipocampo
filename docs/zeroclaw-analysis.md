# ZeroClaw 记忆系统分析

## 概述

ZeroClaw 的记忆系统是 **纯 Rust 实现**，提供统一的 `Memory` trait，支持多种后端。

## 核心组件

### 1. Memory Trait (`traits.rs`)

**定义**:
```rust
#[async_trait]
pub trait Memory: Send + Sync {
    fn name(&self) -> &str;
    
    async fn store(
        &self,
        key: &str,
        content: &str,
        category: MemoryCategory,
        session_id: Option<&str>,
    ) -> anyhow::Result<()>;
    
    async fn recall(
        &self,
        query: &str,
        limit: usize,
        session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>>;
    
    async fn get(&self, key: &str) -> anyhow::Result<Option<MemoryEntry>>;
    
    async fn list(
        &self,
        category: Option<&MemoryCategory>,
        session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>>;
    
    async fn forget(&self, key: &str) -> anyhow::Result<bool>;
    
    async fn count(&self) -> anyhow::Result<usize>;
    
    async fn health_check(&self) -> bool;
}
```

**关键特性**:
- **Async** - 异步接口
- **Send + Sync** - 线程安全
- **统一接口** - 所有后端实现相同 trait

### 2. SqliteMemory (`sqlite.rs`)

**架构**:
```rust
pub struct SqliteMemory {
    conn: Arc<Mutex<Connection>>,  // 线程安全连接
    db_path: PathBuf,
    embedder: Arc<dyn EmbeddingProvider>,
    vector_weight: f32,   // 0.7
    keyword_weight: f32,  // 0.3
    cache_max: usize,     // 10_000
}
```

**SQLite 优化**:
```rust
// WAL 模式 + mmap + cache
conn.execute_batch(
    "PRAGMA journal_mode = WAL;
     PRAGMA synchronous  = NORMAL;
     PRAGMA mmap_size    = 8388608;  // 8 MB
     PRAGMA cache_size   = -2000;    // 2 MB
     PRAGMA temp_store   = MEMORY;"
)?;
```

**Schema**:
```sql
-- 主表
CREATE TABLE memories (
    id          TEXT PRIMARY KEY,
    key         TEXT NOT NULL UNIQUE,
    content     TEXT NOT NULL,
    category    TEXT NOT NULL DEFAULT 'core',
    embedding   BLOB,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

-- FTS5 全文搜索
CREATE VIRTUAL TABLE memories_fts USING fts5(
    key, content, content=memories, content_rowid=rowid
);

-- Embedding 缓存
CREATE TABLE embedding_cache (
    text_hash   TEXT PRIMARY KEY,
    embedding   BLOB NOT NULL,
    created_at  TEXT NOT NULL
);
```

**混合搜索实现**:
```rust
async fn recall(&self, query: &str, limit: usize, session_id: Option<&str>) -> Result<Vec<MemoryEntry>> {
    // 1. 向量搜索
    let query_embedding = self.embedder.embed_one(query).await?;
    let vector_results = self.search_vector(&query_embedding, limit * 2)?;
    
    // 2. 关键词搜索
    let keyword_results = self.search_fts(query, limit * 2, session_id)?;
    
    // 3. 合并结果
    let merged = self.merge_hybrid(vector_results, keyword_results)?;
    
    Ok(merged.into_iter().take(limit).collect())
}
```

### 3. EmbeddingProvider Trait (`embeddings.rs`)

**定义**:
```rust
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    fn name(&self) -> &str;
    fn dimensions(&self) -> usize;
    async fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;
}
```

**实现**:
- `NoopEmbedding` - 无 embedding（仅关键词搜索）
- `OpenAiEmbedding` - OpenAI API
- `OllamaEmbedding` - 本地 Ollama

### 4. 其他后端

#### MarkdownMemory
- 纯文件存储
- 无向量搜索
- 用于简单场景

#### PostgresMemory
- 使用 `pgvector` 扩展
- 全文搜索（`tsvector`）
- 远程存储

#### QdrantMemory
- 专业向量数据库
- 高性能语义搜索

## 数据流

### Store 流程
```
store(key, content, category, session_id)
    ↓
生成 UUID
    ↓
调用 embedder.embed_one(content)
    ↓
BEGIN TRANSACTION
    INSERT INTO memories
    INSERT INTO memories_fts
COMMIT
```

### Recall 流程
```
recall(query, limit, session_id)
    ↓
并行执行:
├─ embedder.embed_one(query)
└─ 搜索 FTS5（BM25）
    ↓
search_vector(embedding, limit * 2)
    ↓
merge_hybrid(vector_results, keyword_results)
    ↓
按分数排序，返回 top K
```

## 关键代码片段

### 向量搜索

```rust
fn search_vector(&self, embedding: &[f32], limit: usize) -> Result<Vec<MemoryEntry>> {
    let conn = self.conn.lock();
    
    let mut stmt = conn.prepare_cached(
        "SELECT id, key, content, category, created_at, embedding
         FROM memories
         WHERE embedding IS NOT NULL
         ORDER BY vector_cosine_similarity(embedding, ?1) DESC
         LIMIT ?2"
    )?;
    
    let rows = stmt.query_map(params![embedding.as_blob(), limit], |row| {
        Ok(MemoryEntry {
            id: row.get(0)?,
            key: row.get(1)?,
            content: row.get(2)?,
            category: row.get(3)?,
            timestamp: row.get(4)?,
            score: Some(cosine_similarity(embedding, &row.get::<_, Vec<f32>>(5)?)),
            ..
        })
    })?;
    
    rows.collect()
}
```

### FTS5 搜索（BM25）

```rust
fn search_fts(&self, query: &str, limit: usize, session_id: Option<&str>) -> Result<Vec<MemoryEntry>> {
    let conn = self.conn.lock();
    
    let mut stmt = conn.prepare_cached(
        "SELECT m.id, m.key, m.content, m.category, m.created_at, fts.rank
         FROM memories m
         JOIN memories_fts fts ON m.rowid = fts.rowid
         WHERE memories_fts MATCH ?1
         ORDER BY bm25(memories_fts) ASC  -- BM25 返回负数，越小越好
         LIMIT ?2"
    )?;
    
    let rows = stmt.query_map(params![query, limit], |row| {
        Ok(MemoryEntry {
            id: row.get(0)?,
            key: row.get(1)?,
            content: row.get(2)?,
            category: row.get(3)?,
            timestamp: row.get(4)?,
            score: Some(bm25_rank_to_score(row.get::<_, f64>(5)?)),
            ..
        })
    })?;
    
    rows.collect()
}
```

### 混合合并

```rust
fn merge_hybrid(
    &self,
    vector_results: Vec<MemoryEntry>,
    keyword_results: Vec<MemoryEntry>,
) -> Result<Vec<MemoryEntry>> {
    let mut scores: HashMap<String, f64> = HashMap::new();
    
    // 向量结果
    for entry in vector_results {
        scores.insert(entry.id.clone(), entry.score.unwrap() * self.vector_weight as f64);
    }
    
    // 关键词结果
    for entry in keyword_results {
        let score = scores.entry(entry.id.clone()).or_insert(0.0);
        *score += entry.score.unwrap() * self.keyword_weight as f64;
    }
    
    // 排序
    let mut merged: Vec<MemoryEntry> = /* 从数据库获取完整条目 */;
    merged.sort_by(|a, b| scores[&b.id].partial_cmp(&scores[&a.id]).unwrap());
    
    Ok(merged)
}
```

## 与 OpenClaw 的对比

| 特性 | ZeroClaw | OpenClaw |
|------|----------|----------|
| **语言** | Rust | TypeScript |
| **存储格式** | SQLite | Markdown + SQLite |
| **并发模型** | Async + Mutex | 文件监听 |
| **Embedding** | Trait 抽象 | Provider 模式 |
| **MMR** | ❌ | ✅ |
| **时间衰减** | ❌ | ✅ |
| **会话隔离** | ✅ (session_id) | ❌ |

## 可复用的设计模式

### 1. Async Trait 抽象

**适用场景**: 需要多种后端实现

```rust
#[async_trait]
pub trait Memory: Send + Sync {
    async fn store(...) -> Result<()>;
    async fn recall(...) -> Result<Vec<MemoryEntry>>;
}
```

### 2. Embedding 缓存

```rust
async fn embed_with_cache(&self, text: &str) -> Result<Vec<f32>> {
    let hash = sha256(text);
    
    // 检查缓存
    if let Some(embedding) = self.get_cached_embedding(&hash).await? {
        return Ok(embedding);
    }
    
    // 调用 API
    let embedding = self.embedder.embed_one(text).await?;
    
    // 写入缓存
    self.cache_embedding(&hash, &embedding).await?;
    
    Ok(embedding)
}
```

### 3. 事务保护

```rust
let conn = self.conn.lock();
let tx = conn.transaction()?;

tx.execute("INSERT INTO memories ...", params![...])?;
tx.execute("INSERT INTO memories_fts ...", params![...])?;

tx.commit()?;
```

---

**分析完成时间**: 2026-03-04
