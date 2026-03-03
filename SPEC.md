# SPEC.md - Hipocampo 功能与设计规范

> **状态**: 草案 v0.1
> **最后更新**: 2026-03-04
> **负责人**: Lead Agent (main)

---

## 1. 项目愿景

Hipocampo 是一个 **Agent 优先** 的统一记忆后端，为 OpenClaw (TypeScript) 和 ZeroClaw (Rust) 提供兼容的记忆服务。

**核心原则**：
- **Markdown 是 source of truth**，向量数据库是派生索引
- **人类可读可调试**，禁止黑盒
- **Agent 是第一优先级**，这不是数据库项目
- **独立后端**，与 Agent 实现解耦

---

## 2. 功能需求

### 2.1 核心功能

#### 2.1.1 记忆存储
- **格式**: Markdown 文件（MEMORY.md + memory/YYYY-MM-DD.md）
- **类别**: Core（长期）、Daily（每日）、Conversation（对话）
- **元数据**: timestamp、session_id、content_hash

#### 2.1.2 记忆搜索
- **混合搜索**: Dense Vector (语义) + BM25 (关键词) + RRF (重排序)
- **过滤**: 按类别、时间范围、session_id
- **排序**: 相关性分数、时间戳

#### 2.1.3 分层机制
```
热记忆 (Hot)    → 内存缓存（LRU，可选）
短期记忆 (STM)  → 本地 SQLite
长期记忆 (LTM)  → 本地 SQLite + 远程 PostgreSQL（可选）
```

**转存规则**：
- Daily 记忆 7 天后自动归档
- Core 记忆永不过期
- Conversation 记忆 24 小时后清理

#### 2.1.4 向量索引
- **存储**: SQLite (sqlite-vec) 或 PostgreSQL (pgvector)
- **Provider**: OpenAI、Google Gemini、Voyage、Local (Ollama)
- **去重**: SHA-256 content hash
- **增量更新**: 只处理新增/修改内容

### 2.2 兼容层

#### 2.2.1 OpenClaw 兼容
- **接口**: `memory_search` + `memory_get` 工具
- **配置**: 兼容 `memorySearch.*` 配置项
- **数据格式**: 兼容 OpenClaw 的 Markdown 布局

#### 2.2.2 ZeroClaw 兼容
- **接口**: `Memory` trait (Rust)
- **配置**: 兼容 `memory.backend` 配置项
- **数据格式**: 兼容 ZeroClaw 的 SQLite schema

---

## 3. 技术设计

### 3.1 架构

```
┌─────────────────────────────────────┐
│         Agent Layer                 │
│  (OpenClaw TS / ZeroClaw Rust)      │
└─────────────┬───────────────────────┘
              │
┌─────────────▼───────────────────────┐
│    Compatibility Layer              │
│  - OpenClaw Adapter (TS)            │
│  - ZeroClaw Adapter (Rust)          │
└─────────────┬───────────────────────┘
              │
┌─────────────▼───────────────────────┐
│      Hipocampo Core (Rust)          │
│  - Memory Trait                     │
│  - Embedding Provider Trait         │
│  - Search Engine                    │
└─────────────┬───────────────────────┘
              │
┌─────────────▼───────────────────────┐
│      Storage Layer                  │
│  - SQLite (sqlite-vec)              │
│  - PostgreSQL (pgvector, optional)  │
└─────────────────────────────────────┘
              │
┌─────────────▼───────────────────────┐
│     Markdown Layer (source of truth)│
│  - MEMORY.md                        │
│  - memory/YYYY-MM-DD.md             │
└─────────────────────────────────────┘
```

### 3.2 核心数据结构

#### MemoryEntry
```rust
pub struct MemoryEntry {
    pub id: String,              // UUID
    pub key: String,             // 唯一标识
    pub content: String,         // 记忆内容
    pub category: MemoryCategory,// core/daily/conversation
    pub timestamp: String,       // ISO 8601
    pub session_id: Option<String>,
    pub score: Option<f64>,      // 搜索相关性
    pub embedding: Option<Vec<f32>>,
}
```

#### Chunk（索引单元）
```rust
pub struct Chunk {
    pub content_hash: String,    // SHA-256 (16 chars)
    pub content: String,
    pub source: String,          // 文件路径
    pub start_line: usize,
    pub end_line: usize,
    pub heading: String,
    pub heading_level: u8,
}
```

### 3.3 搜索流程

```
Query
  ↓
Embed Query
  ↓
┌─────────────┐
│ Hybrid Search│
│ - Vector     │
│ - BM25       │
└─────────────┘
  ↓
RRF Rerank (k=60)
  ↓
Top-K Results
  ↓
从 Markdown 读取完整内容
```

### 3.4 索引流程

```
Markdown Files
  ↓
Scanner (文件监听)
  ↓
Chunker (按标题分块)
  ↓
Dedup (SHA-256)
  ↓
Embedding (增量)
  ↓
SQLite / PostgreSQL
```

---

## 4. API 设计

### 4.1 Rust Core API

```rust
pub trait Memory: Send + Sync {
    async fn store(&self, key: &str, content: &str, category: MemoryCategory, session_id: Option<&str>) -> Result<String>;
    async fn search(&self, query: &str, opts: SearchOptions) -> Result<Vec<MemoryEntry>>;
    async fn get(&self, key: &str) -> Result<Option<MemoryEntry>>;
    async fn list(&self, filter: ListFilter) -> Result<Vec<MemoryEntry>>;
    async fn forget(&self, key: &str) -> Result<bool>;
    async fn count(&self) -> Result<usize>;
    async fn health_check(&self) -> bool;
}
```

### 4.2 OpenClaw 兼容 API

```typescript
interface OpenClawMemory {
  memory_search(query: string, opts?: SearchOptions): Promise<MemorySearchResult[]>;
  memory_get(relPath: string, from?: number, lines?: number): Promise<{text: string, path: string}>;
}
```

### 4.3 CLI API

```bash
hipocampo index [--paths PATH...]          # 索引 markdown 文件
hipocampo search "query" [--limit 10]      # 搜索记忆
hipocampo compact [--category daily]       # 压缩归档
hipocampo status                           # 系统状态
hipocampo migrate --from openclaw          # 数据迁移
```

---

## 5. 性能指标

| 指标 | 目标 |
|------|------|
| 搜索延迟 (P95) | < 100ms |
| 索引吞吐量 | > 1000 chunks/s |
| 内存占用 (热记忆) | < 100MB |
| 存储空间 (10k 记忆) | < 500MB |

---

## 6. 质量标准

- **测试覆盖率**: > 80%
- **测试状态**: 所有测试必须全绿通过
- **文档完整性**: API 文档 + 集成指南 + 架构图
- **代码审查**: 所有代码必须经过 reviewer 审核
- **可调试性**: 提供详细的日志和诊断工具

---

**下一步**：
- [ ] 研究 OpenClaw 记忆系统实现细节
- [ ] 研究 ZeroClaw 记忆系统实现细节
- [ ] 研究 memsearch 分层机制
- [ ] 设计详细的技术方案
- [ ] 创建 AGENT.md 和 SCOPE.md
