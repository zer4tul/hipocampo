# Hipocampo - 统一记忆后端

> **Agent 优先**的记忆后端，兼容 OpenClaw 和 ZeroClaw

---

## 项目状态

**当前阶段**: Phase 1 - 研究与设计（已完成）

**进度**：
- ✅ SPEC.md - 功能与设计规范
- ✅ AGENT.md - 行为与开发流程约束
- ✅ SCOPE.md - 硬边界与禁止项
- ✅ OpenClaw 记忆系统分析
- ✅ ZeroClaw 记忆系统分析
- ✅ memsearch 分层机制分析

---

## 核心原则

### 1. Markdown 是 Source of Truth

- 所有记忆存储在 Markdown 文件中
- 向量数据库是派生索引，可随时重建
- 人类可读、可调试、可版本控制

### 2. Agent 优先

- **这不是数据库项目**，这是 Agent 服务项目
- 任何设计决策优先考虑 Agent 使用体验
- 如有疑问，在调研阶段找用户澄清

### 3. 人类可读可调试

- 禁止黑盒设计
- 提供详细日志和诊断工具
- 所有操作都有对应的查询命令

### 4. 质量保证

- 测试覆盖率 > 80%
- 所有测试必须全绿通过
- 代码必须经过 reviewer 审核

---

## 架构

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

---

## 核心功能

### 1. 记忆存储

- **格式**: Markdown 文件
- **类别**: Core / Daily / Conversation
- **Session 隔离**: 支持 session_id 过滤

### 2. 混合搜索

- **Vector Search**: 语义匹配（Dense embeddings）
- **BM25 Search**: 关键词匹配（FTS5）
- **RRF Rerank**: Reciprocal Rank Fusion (k=60)
- **MMR**: 多样性重排序（可选）
- **时间衰减**: 新记忆优先（半衰期 30 天）

### 3. 分层存储

```
热记忆 (Hot)    → 内存缓存（LRU，可选）
短期记忆 (STM)  → 本地 SQLite
长期记忆 (LTM)  → 本地 SQLite + 远程 PostgreSQL（可选）
```

---

## API 设计

### Rust Core API

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

### CLI API

```bash
hipocampo index [--paths PATH...]          # 索引 markdown 文件
hipocampo search "query" [--limit 10]      # 搜索记忆
hipocampo compact [--category daily]       # 压缩归档
hipocampo status                           # 系统状态
```

---

## 性能目标

| 指标 | 目标 |
|------|------|
| 搜索延迟 (P95) | < 100ms |
| 索引吞吐量 | > 1000 chunks/s |
| 内存占用 (热记忆) | < 100MB |
| 存储空间 (10k 记忆) | < 500MB |

---

## 文档

- [SPEC.md](./SPEC.md) - 功能与设计规范
- [AGENT.md](./AGENT.md) - 行为与开发流程约束
- [SCOPE.md](./SCOPE.md) - 硬边界与禁止项
- [research/](./research/) - 研究文档
  - [openclaw-analysis.md](./research/openclaw-analysis.md)
  - [zeroclaw-analysis.md](./research/zeroclaw-analysis.md)
  - [memsearch-analysis.md](./research/memsearch-analysis.md)

---

## 开发路线

### Phase 1: 研究与设计（✅ 已完成）

- [x] 研究 OpenClaw 记忆系统
- [x] 研究 ZeroClaw 记忆系统
- [x] 研究 memsearch 分层机制
- [x] 创建核心文档（SPEC/AGENT/SCOPE）

### Phase 2: 核心实现（进行中）

- [ ] 实现 Memory trait（Rust）
- [ ] 实现 SQLite 后端
- [ ] 实现混合搜索
- [ ] 实现分层存储

### Phase 3: 兼容层

- [ ] OpenClaw adapter (TypeScript)
- [ ] ZeroClaw adapter (Rust)
- [ ] 兼容性测试

### Phase 4: 测试与优化

- [ ] 完善测试用例（覆盖率 > 80%）
- [ ] 性能测试
- [ ] 文档完善

---

## 技术栈

- **Core**: Rust
- **Storage**: SQLite + sqlite-vec
- **Embedding**: OpenAI / Google Gemini / Voyage / Local (Ollama)
- **Search**: Vector + BM25 + RRF + MMR
- **CLI**: clap

---

## License

MIT

---

**项目启动时间**: 2026-03-04
**当前版本**: 0.1.0
**负责人**: Lead Agent (main)
