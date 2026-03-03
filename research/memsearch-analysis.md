# memsearch 分层机制分析

> **研究时间**: 2026-03-04
> **研究员**: Lead Agent (main)
> **来源**: ~/code-base/memsearch/

---

## 核心设计理念

### Markdown 作为 Source of Truth

memsearch 的核心原则：**Markdown 文件是 canonical data store**，向量数据库是派生索引。

**优势**：
1. **人类可读**：任何文本编辑器都能查看
2. **Git 友好**：完整的版本历史、diff、merge
3. **零供应商锁定**：纯文本格式，易于迁移
4. **可移植性**：复制文件即可，无需导出

**对比数据库作为 source of truth**：
- 数据库文件是二进制 blob，需要特定软件
- 迁移成本高，格式不兼容
- 备份复杂，版本控制困难

---

## 分层存储架构

### 三层部署模式

memsearch 支持三种 Milvus 部署模式：

```
┌─────────────────────────────────────┐
│        milvus_uri 配置              │
└─────────────┬───────────────────────┘
              │
    ┌─────────┴─────────┬─────────────┐
    │                   │             │
┌───▼────┐      ┌──────▼──────┐  ┌───▼─────────┐
│  Lite  │      │   Server    │  │ Zilliz Cloud│
│  本地   │      │  自托管      │  │  云托管     │
└────────┘      └─────────────┘  └─────────────┘
```

| Tier | URI Pattern | 用途 |
|------|-------------|------|
| **Milvus Lite** | `~/.memsearch/milvus.db` | 个人使用、单 Agent、开发 |
| **Milvus Server** | `http://localhost:19530` | 多 Agent 团队、共享基础设施 |
| **Zilliz Cloud** | `https://...zillizcloud.com` | 生产 SaaS、零运维 |

### 物理隔离策略

**所有 Agent 和项目共享同一个 collection name（`memsearch_chunks`）**。

**物理隔离通过不同的 `milvus_uri` 实现**：
- 每个 Agent 有独立的 Milvus Lite 文件
- 或独立的 Milvus Server
- 或独立的 Zilliz Cloud 集群

**优势**：避免多租户 collection 管理的复杂性

---

## 数据流

### 索引流程

```
Markdown Files (MEMORY.md, memory/YYYY-MM-DD.md)
    ↓
Scanner (文件监听)
    ↓
Chunker (按标题分块)
    ↓
Dedup (SHA-256 content hash)
    ↓
Embedding (增量，只处理新增/修改)
    ↓
SQLite / Milvus
```

### 搜索流程

```
Query
    ↓
Embed Query
    ↓
Hybrid Search
├─ Vector Search (cosine similarity)
└─ BM25 Search (keyword matching)
    ↓
RRF Rerank (k=60)
    ↓
Top-K Results
    ↓
从 Markdown 读取完整内容
```

### Compact 循环

```
Indexed Chunks in Milvus
    ↓
Retrieve all (or filtered)
    ↓
LLM Summarize
    ↓
Append to memory/YYYY-MM-DD.md
    ↓
File Watcher detects change
    ↓
Auto re-index updated file
    ↓
(Loop back to Indexed Chunks)
```

---

## 分块策略

### 基于标题的分块

将 Markdown 标题（`#` 到 `######`）作为自然的 chunk 边界。

```
# Project Notes          ← preamble chunk

Some introductory text.

## Redis Configuration   ← chunk boundary

We chose Redis for caching...

### Connection Settings  ← chunk boundary

host=localhost, port=6379...

## Authentication        ← chunk boundary

We use JWT tokens...
```

### 超大段落的段落分割

当一个标题段落超过 `max_chunk_size`（默认 1500 字符）时：
- 在段落边界（空行）分割
- 保留 `overlap_lines`（默认 2 行）以保持上下文连续性

### Chunk 元数据

| 字段 | 描述 |
|------|------|
| `content` | chunk 的原始文本 |
| `source` | 绝对文件路径 |
| `heading` | 最近标题文本 |
| `heading_level` | 标题深度（0-6） |
| `start_line` | 起始行号（1-indexed） |
| `end_line` | 结束行号 |
| `content_hash` | SHA-256 哈希（16 字符） |

---

## 去重机制

### 内容寻址存储

使用 **SHA-256 content hash** 作为主键，避免重复调用 embedding API。

**流程**：
1. 计算 chunk content 的 SHA-256（截断为 16 字符）
2. 计算复合 chunk ID：`hash(source:lines:contentHash:model)`
3. 查询 Milvus 检查 ID 是否存在
4. 只对不存在的 chunk 调用 embedding API
5. 删除不再出现的 stale chunks

**优势**：
- 无需外部缓存（hash 即主键）
- 增量索引（只处理新内容）
- 节省 API 成本

---

## 混合搜索

### Dense + BM25 + RRF

**三种搜索策略**：
1. **Dense Vector Search**：cosine similarity（语义匹配）
2. **BM25 Sparse Search**：keyword matching（精确术语）
3. **RRF Reranking**：Reciprocal Rank Fusion（k=60）合并结果

**优势**：
- 捕获纯语义搜索可能错过的结果（精确名称、错误代码、配置值）
- 仍然受益于语义理解

---

## 配置系统

### 四层配置

```
1. Defaults (hardcoded)
    ↓
2. Global Config (~/.memsearch/config.toml)
    ↓
3. Project Config (.memsearch.toml)
    ↓
4. CLI Flags (--milvus-uri, etc.)
```

### 配置示例

```toml
[milvus]
uri = "~/.memsearch/milvus.db"
collection = "memsearch_chunks"

[embedding]
provider = "openai"
model = ""  # empty = provider default

[chunking]
max_chunk_size = 1500
overlap_lines = 2

[watch]
debounce_ms = 1500
```

---

## 可复用的设计模式

### 1. Markdown 作为 Source of Truth

**适用场景**：需要人类可读、可调试、可版本控制的知识库

**实现**：
```rust
// 所有数据都存储在 Markdown 文件中
// 向量数据库只是派生索引，可以随时重建
fn rebuild_index_from_markdown() -> Result<()> {
    let files = scan_markdown_files()?;
    let chunks = chunk_files(files)?;
    for chunk in chunks {
        let embedding = embed_chunk(&chunk)?;
        upsert_to_db(chunk, embedding)?;
    }
    Ok(())
}
```

### 2. 内容寻址去重

**适用场景**：避免重复调用 embedding API

**实现**：
```rust
fn dedup_chunk(chunk: &Chunk) -> bool {
    let hash = sha256(&chunk.content);
    let chunk_id = format!("{}:{}:{}:{}:{}",
        chunk.source, chunk.start_line, chunk.end_line, hash, model
    );
    !db.exists(&chunk_id)
}
```

### 3. 混合搜索 + RRF

**适用场景**：平衡语义搜索和关键词搜索

**实现**：
```rust
fn hybrid_search(query: &str) -> Vec<Result> {
    let query_embedding = embed(query);
    let vector_results = search_vector(&query_embedding);
    let keyword_results = search_bm25(query);
    rrf_rerank(vector_results, keyword_results, k=60)
}
```

### 4. 文件监听 + 自动索引

**适用场景**：Markdown 文件频繁更新

**实现**：
```rust
fn watch_and_index() {
    let mut watcher = Watcher::new(debounce_ms=1500);
    watcher.on_change(|file| {
        let chunks = chunk_file(file);
        let new_chunks = dedup(chunks);
        embed_and_upsert(new_chunks);
    });
}
```

---

## 与 Hipocampo 的相关性

| memsearch 特性 | Hipocampo 应用 |
|----------------|----------------|
| Markdown 作为 source of truth | ✅ 直接采用 |
| 内容寻址去重 | ✅ 直接采用 |
| 混合搜索 + RRF | ✅ 直接采用 |
| 分层存储（Lite/Server/Cloud）| ✅ 借鉴设计（SQLite/PostgreSQL） |
| 文件监听 + 自动索引 | ✅ 直接采用 |
| Compact 循环 | ⚠️ 可选功能 |

---

**研究完成时间**: 2026-03-04
