# OpenClaw 记忆系统分析

## 概述

OpenClaw 的记忆系统基于 **Markdown 文件 + 向量索引**，支持混合搜索（BM25 + Vector）。

## 核心组件

### 1. MemoryIndexManager (`manager.ts`)

**职责**:
- 管理 SQLite 索引
- 协调文件监听和同步
- 提供搜索接口

**关键方法**:
```typescript
class MemoryIndexManager implements MemorySearchManager {
  // 搜索（混合搜索）
  async search(query: string, opts?: SearchOptions): Promise<MemorySearchResult[]>
  
  // 读取文件
  async readFile(params: { relPath: string; from?: number; lines?: number }): Promise<{ text: string; path: string }>
  
  // 同步索引
  async sync(params?: { reason?: string; force?: boolean }): Promise<void>
  
  // 状态查询
  status(): MemoryProviderStatus
}
```

**数据流**:
```
Markdown 文件
    ↓ (文件监听)
标记为 dirty
    ↓ (同步触发)
分块 → Embedding → 存储 SQLite
    ↓ (搜索请求)
混合查询 → MMR → 返回结果
```

### 2. Embedding Provider (`embeddings.ts`)

**支持的 Provider**:
- OpenAI (`text-embedding-3-small`)
- Gemini (`gemini-embedding-001`)
- Voyage (`voyage-3`)
- Mistral (`mistral-embed`)
- Local (`node-llama-cpp`)

**核心接口**:
```typescript
interface EmbeddingProvider {
  name(): string;
  dimensions(): number;
  embed(texts: string[]): Promise<number[][]>;
}
```

### 3. Hybrid Search (`hybrid.ts`)

**算法**:
```typescript
function mergeHybridResults(
  vectorResults: SearchResult[],
  keywordResults: SearchResult[],
  vectorWeight: number,
  keywordWeight: number
): SearchResult[] {
  // 合并结果，计算加权分数
  finalScore = vectorWeight * vectorScore + keywordWeight * keywordScore
}
```

**关键特性**:
- BM25 全文搜索（FTS5）
- 向量余弦相似度
- 加权融合
- 候选池扩展（`candidateMultiplier`）

### 4. Backend Config (`backend-config.ts`)

**支持的 Backend**:
- `builtin` - 内置 SQLite
- `qmd` - QMD sidecar
- `lancedb` - LanceDB
- `hybrid` - 混合后端

**配置示例**:
```json5
memory: {
  backend: "builtin",
  citations: "auto",
  memorySearch: {
    provider: "openai",
    model: "text-embedding-3-small",
    query: {
      hybrid: {
        enabled: true,
        vectorWeight: 0.7,
        textWeight: 0.3,
        mmr: { enabled: true, lambda: 0.7 },
        temporalDecay: { enabled: true, halfLifeDays: 30 }
      }
    }
  }
}
```

## 数据结构

### MemorySearchResult

```typescript
type MemorySearchResult = {
  path: string;        // 文件路径
  startLine: number;  // 起始行
  endLine: number;    // 结束行
  score: number;      // 相关性分数
  snippet: string;    // 文本片段
  source: "memory" | "sessions";
  citation?: string;
};
```

### SQLite Schema

```sql
-- 主表
CREATE TABLE chunks (
  id INTEGER PRIMARY KEY,
  path TEXT NOT NULL,
  startLine INTEGER NOT NULL,
  endLine INTEGER NOT NULL,
  text TEXT NOT NULL,
  embedding BLOB,
  created_at TEXT,
  updated_at TEXT
);

-- FTS5 虚拟表
CREATE VIRTUAL TABLE chunks_fts USING fts5(
  path, text, content=chunks, content_rowid=rowid
);

-- 向量表（sqlite-vec）
CREATE VIRTUAL TABLE chunks_vec USING vec0(
  embedding FLOAT[1536]
);
```

## 可复用的设计模式

### 1. 混合搜索策略

**适用场景**: 需要同时支持语义搜索和关键词搜索

**实现**:
```typescript
const vectorResults = await searchVector(query, embedding);
const keywordResults = await searchKeyword(query);
const merged = mergeHybridResults(vectorResults, keywordResults, 0.7, 0.3);
```

### 2. 文件监听 + 增量同步

**适用场景**: 记忆文件频繁更新

**实现**:
```typescript
const watcher = chokidar.watch(memoryDir);
watcher.on('change', () => {
  dirty = true;
  scheduleSync();
});
```

### 3. Embedding 缓存

**适用场景**: 避免重复调用 embedding API

**实现**:
```typescript
const cacheKey = hash(text);
const cached = await db.get('SELECT embedding FROM cache WHERE key = ?', [cacheKey]);
if (cached) return cached.embedding;

const embedding = await provider.embed(text);
await db.run('INSERT INTO cache (key, embedding) VALUES (?, ?)', [cacheKey, embedding]);
```

### 4. 降级策略

**适用场景**: 向量搜索不可用时降级到关键词搜索

**实现**:
```typescript
if (!vectorAvailable) {
  console.warn('Vector search unavailable, falling back to keyword search');
  return searchKeyword(query);
}
```

## 关键代码片段

### 分块算法

```typescript
function chunkText(text: string, targetSize = 400, overlap = 80): Chunk[] {
  const chunks: Chunk[] = [];
  const paragraphs = text.split(/\n\n+/);
  
  let current = '';
  let startLine = 1;
  
  for (const para of paragraphs) {
    if (current.length + para.length > targetSize && current.length > 0) {
      chunks.push({ text: current, startLine, endLine: startLine + current.split('\n').length });
      current = para;
      startLine += current.split('\n').length - overlap;
    } else {
      current += '\n\n' + para;
    }
  }
  
  return chunks;
}
```

### MMR 重排序

```typescript
function mmrRerank(results: SearchResult[], lambda = 0.7, topK = 10): SearchResult[] {
  const selected: SearchResult[] = [];
  const remaining = [...results];
  
  while (selected.length < topK && remaining.length > 0) {
    let bestScore = -Infinity;
    let bestIdx = 0;
    
    for (let i = 0; i < remaining.length; i++) {
      const relevance = remaining[i].score;
      const diversity = Math.max(...selected.map(s => jaccard(s.text, remaining[i].text)));
      const score = lambda * relevance - (1 - lambda) * diversity;
      
      if (score > bestScore) {
        bestScore = score;
        bestIdx = i;
      }
    }
    
    selected.push(remaining.splice(bestIdx, 1)[0]);
  }
  
  return selected;
}
```

## 与 ZeroClaw 的差异

| 特性 | OpenClaw | ZeroClaw |
|------|----------|----------|
| **存储格式** | Markdown 文件 | SQLite / PostgreSQL |
| **索引方式** | SQLite 索引 | 内嵌 SQLite |
| **搜索接口** | `search()` + `readFile()` | `recall()` + `get()` |
| **混合搜索** | ✅ BM25 + Vector | ✅ BM25 + Vector |
| **MMR** | ✅ | ❌ |
| **时间衰减** | ✅ | ❌ |
| **多后端** | ✅ (QMD, LanceDB) | ✅ (Postgres, Qdrant) |

---

**分析完成时间**: 2026-03-04
