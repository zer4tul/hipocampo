# Hipocampo 架构设计

## 概述

Hipocampo 是一个统一的记忆后端，设计目标：
- **兼容 OpenClaw (TypeScript) 和 ZeroClaw (Rust)**
- **多后端支持**: SQLite、PostgreSQL、Markdown
- **向量搜索**: 支持多种 embedding provider
- **混合搜索**: BM25 + Vector

## 核心抽象

### 1. Memory Trait (统一接口)

```rust
/// Rust 版本
#[async_trait]
pub trait Memory: Send + Sync {
    /// 后端名称
    fn name(&self) -> &str;
    
    /// 存储记忆
    async fn store(&self, key: &str, content: &str, category: MemoryCategory, session_id: Option<&str>) -> Result<()>;
    
    /// 搜索记忆（混合搜索）
    async fn search(&self, query: &str, opts: SearchOptions) -> Result<Vec<MemoryEntry>>;
    
    /// 获取单条记忆
    async fn get(&self, key: &str) -> Result<Option<MemoryEntry>>;
    
    /// 列出记忆
    async fn list(&self, filter: ListFilter) -> Result<Vec<MemoryEntry>>;
    
    /// 删除记忆
    async fn forget(&self, key: &str) -> Result<bool>;
    
    /// 健康检查
    async fn health_check(&self) -> bool;
}

/// 记忆条目
pub struct MemoryEntry {
    pub id: String,
    pub key: String,
    pub content: String,
    pub category: MemoryCategory,
    pub timestamp: String,
    pub session_id: Option<String>,
    pub score: Option<f64>,  // 搜索相关性分数
    pub embedding: Option<Vec<f32>>,  // 向量
}

/// 记忆类别
pub enum MemoryCategory {
    Core,         // 长期记忆
    Daily,        // 每日日志
    Conversation, // 对话上下文
    Custom(String),
}
```

```typescript
/// TypeScript 版本
export interface Memory {
  name(): string;
  
  store(key: string, content: string, category: MemoryCategory, sessionId?: string): Promise<void>;
  search(query: string, opts: SearchOptions): Promise<MemoryEntry[]>;
  get(key: string): Promise<MemoryEntry | null>;
  list(filter: ListFilter): Promise<MemoryEntry[]>;
  forget(key: string): Promise<boolean>;
  healthCheck(): Promise<boolean>;
}

export interface MemoryEntry {
  id: string;
  key: string;
  content: string;
  category: MemoryCategory;
  timestamp: string;
  sessionId?: string;
  score?: number;
  embedding?: number[];
}

export type MemoryCategory = 'core' | 'daily' | 'conversation' | string;
```

### 2. Embedding Provider Trait

```rust
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    fn name(&self) -> &str;
    fn dimensions(&self) -> usize;
    async fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;
    async fn embed_one(&self, text: &str) -> Result<Vec<f32>> {
        self.embed(&[text]).await?.pop().unwrap()
    }
}
```

### 3. 后端实现

#### SQLite Backend

**复用 ZeroClaw 的设计**:
- WAL 模式 + mmap
- FTS5 全文搜索（BM25）
- 向量存储为 BLOB
- Embedding 缓存

#### PostgreSQL Backend

**新设计**:
- 使用 pgvector 扩展
- 全文搜索（tsvector）
- 异步连接池

#### Markdown Backend

**复用 OpenClaw 的设计**:
- 文件格式: MEMORY.md + memory/YYYY-MM-DD.md
- 向量索引存储在 SQLite
- 支持增量读取

## 混合搜索策略

### 评分公式

```
finalScore = vectorWeight * vectorScore + keywordWeight * keywordScore
```

默认权重:
- `vectorWeight = 0.7`
- `keywordWeight = 0.3`

### 后处理

1. **MMR 重排序**（可选）: 去除重复结果
2. **时间衰减**（可选）: 新记忆优先

## 项目结构

```
hipocampo/
├── core/                     # 核心抽象（Rust + TS 定义）
│   ├── trait.rs             # Memory trait
│   ├── types.rs             # 公共类型
│   └── trait.ts             # TypeScript 接口
├── rust/                     # Rust 实现
│   ├── sqlite/              # SQLite 后端
│   ├── postgres/            # PostgreSQL 后端
│   ├── markdown/            # Markdown 后端
│   └── embeddings/          # Embedding providers
├── typescript/               # TypeScript 实现
│   ├── sqlite/              # SQLite 后端
│   ├── postgres/            # PostgreSQL 后端
│   ├── markdown/            # Markdown 后端
│   └── embeddings/          # Embedding providers
├── docs/                     # 文档
└── tests/                    # 测试
    ├── integration/         # 集成测试
    └── compatibility/       # OpenClaw/ZeroClaw 兼容性测试
```

## 兼容性策略

### OpenClaw 集成

1. **替换 `src/memory/` 模块**
2. **保持 `memory_search` 和 `memory_get` 工具接口不变**
3. **配置格式兼容**: `memorySearch.provider`, `memorySearch.query.hybrid`

### ZeroClaw 集成

1. **保持 `Memory` trait 签名不变**
2. **后端切换通过配置**: `memory.backend = "sqlite"`
3. **数据格式兼容**: SQLite schema 保持一致

## 技术栈

### Rust 侧
- `tokio` - 异步运行时
- `rusqlite` + `sqlite-vec` - SQLite + 向量
- `sqlx` + `pgvector` - PostgreSQL
- `reqwest` - HTTP client（embeddings API）

### TypeScript 侧
- `better-sqlite3` + `sqlite-vec` - SQLite
- `pg` + `pgvector` - PostgreSQL
- `node-fetch` - HTTP client

## 实施路线

### Phase 1: 核心抽象（1 天）
- [ ] 定义 Memory trait（Rust + TS）
- [ ] 定义 EmbeddingProvider trait
- [ ] 定义公共类型

### Phase 2: SQLite 后端（2 天）
- [ ] Rust 实现（复用 ZeroClaw）
- [ ] TypeScript 实现（复用 OpenClaw）
- [ ] 测试：数据一致性

### Phase 3: 混合搜索（1 天）
- [ ] BM25 + Vector 混合
- [ ] MMR 重排序
- [ ] 时间衰减

### Phase 4: PostgreSQL 后端（1 天）
- [ ] Rust 实现
- [ ] TypeScript 实现
- [ ] 测试：pgvector 集成

### Phase 5: 兼容性测试（1 天）
- [ ] OpenClaw 集成测试
- [ ] ZeroClaw 集成测试
- [ ] 性能基准

### Phase 6: 文档与发布（1 天）
- [ ] API 文档
- [ ] 集成指南
- [ ] 发布到 crates.io / npm

---

**预计总时间**: 7 天
