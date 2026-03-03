# ZeroClaw 记忆系统分析

> **研究时间**: 2026-03-04
> **研究员**: Lead Agent (main)
> **来源**: ~/code-base/zeroclaw/src/memory/

---

## 核心架构

### Memory Trait（统一接口）

```rust
#[async_trait]
pub trait Memory: Send + Sync {
    fn name(&self) -> &str;

    // CRUD 操作
    async fn store(&self, key: &str, content: &str, category: MemoryCategory, session_id: Option<&str>) -> Result<()>;
    async fn recall(&self, query: &str, limit: usize, session_id: Option<&str>) -> Result<Vec<MemoryEntry>>;
    async fn get(&self, key: &str) -> Result<Option<MemoryEntry>>;
    async fn list(&self, category: Option<&MemoryCategory>, session_id: Option<&str>) -> Result<Vec<MemoryEntry>>;
    async fn forget(&self, key: &str) -> Result<bool>;

    // 元数据
    async fn count(&self) -> Result<usize>;
    async fn health_check(&self) -> bool;
}
```

**关键特性**：
- **Async + Send + Sync**: 线程安全异步
- **Session 隔离**: 支持 `session_id` 过滤
- **Category 分类**: Core/Daily/Conversation

---

## 数据结构

### MemoryEntry

```rust
pub struct MemoryEntry {
    pub id: String,
    pub key: String,
    pub content: String,
    pub category: MemoryCategory,
    pub timestamp: String,
    pub session_id: Option<String>,
    pub score: Option<f64>,  // 搜索相关性
}
```

### MemoryCategory

```rust
pub enum MemoryCategory {
    Core,          // 长期记忆
    Daily,         // 每日日志
    Conversation,  // 对话上下文
    Custom(String),// 自定义
}
```

---

## Chunking 策略

### 基于标题的分块

**chunker.rs** 实现：

```rust
pub fn chunk_markdown(text: &str, max_tokens: usize) -> Vec<Chunk> {
    // 策略：
    // 1. 按 ## 和 # 标题分割
    // 2. 如果段落超过 max_tokens，按空行分割
    // 3. 如果仍然超过，按行分割

    let sections = split_on_headings(text);
    // ...
}

struct Chunk {
    index: usize,
    content: String,
    heading: Option<Rc<str>>,
}
```

**Token 估算**：~4 字符 = 1 token

---

## SQLite 后端

### Schema（从 sqlite.rs 推断）

```sql
CREATE TABLE memories (
    id          TEXT PRIMARY KEY,
    key         TEXT NOT NULL UNIQUE,
    content     TEXT NOT NULL,
    category    TEXT NOT NULL DEFAULT 'core',
    embedding   BLOB,              -- 向量
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE VIRTUAL TABLE memories_fts USING fts5(
    key, content, content=memories, content_rowid=rowid
);
```

### 混合搜索

```rust
async fn recall(&self, query: &str, limit: usize, session_id: Option<&str>) -> Result<Vec<MemoryEntry>> {
    // 1. 向量搜索
    let vector_results = self.search_vector(&query_embedding, limit * 2)?;

    // 2. FTS5 搜索（BM25）
    let keyword_results = self.search_fts(query, limit * 2, session_id)?;

    // 3. 合并
    let merged = self.merge_hybrid(vector_results, keyword_results)?;

    Ok(merged.into_iter().take(limit).collect())
}
```

---

## 可复用的设计模式

### 1. Async Trait 抽象

**适用场景**: 多后端实现

```rust
#[async_trait]
pub trait Memory: Send + Sync {
    async fn store(...) -> Result<()>;
    async fn recall(...) -> Result<Vec<MemoryEntry>>;
}
```

### 2. Session 隔离

**实现**:
```rust
async fn recall(&self, query: &str, session_id: Option<&str>) -> Result<Vec<MemoryEntry>> {
    let sql = if let Some(sid) = session_id {
        "SELECT * FROM memories WHERE session_id = ? AND content LIKE ?"
    } else {
        "SELECT * FROM memories WHERE content LIKE ?"
    };
    // ...
}
```

### 3. Category 分类

```rust
pub enum MemoryCategory {
    Core,         // 永久
    Daily,        // 7天后归档
    Conversation, // 24小时后清理
}
```

---

## 与 OpenClaw 的差异

| 特性 | ZeroClaw | OpenClaw |
|------|----------|----------|
| **语言** | Rust | TypeScript |
| **存储** | SQLite | Markdown + SQLite |
| **搜索接口** | `recall()` | `search()` |
| **Session 隔离** | ✅ | ❌ |
| **MMR** | ❌ | ✅ |
| **时间衰减** | ❌ | ✅ |
| **Chunking** | 标题 + 段落 | 标题 + 段落 |

---

## Hipocampo 可复用的设计

✅ **Memory trait**（统一接口）
✅ **MemoryCategory**（分类机制）
✅ **Session 隔离**
✅ **混合搜索**（Vector + BM25）
✅ **Chunking 策略**（标题 + 段落）

⚠️ **需要增加**：
- MMR 重排序
- 时间衰减
- Embedding 缓存

---

**研究完成时间**: 2026-03-04
