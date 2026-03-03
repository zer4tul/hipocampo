# OpenClaw 记忆系统分析

> **研究时间**: 2026-03-04
> **研究员**: Lead Agent (main)
> **来源**: ~/code-base/openclaw/src/memory/

---

## 核心架构

### MemorySearchManager 接口

```typescript
export interface MemorySearchManager {
  search(query: string, opts?: SearchOptions): Promise<MemorySearchResult[]>;
  readFile(params: { relPath: string; from?: number; lines?: number }): Promise<{ text: string; path: string }>;
  status(): MemoryProviderStatus;
  sync?(params?: { reason?: string; force?: boolean }): Promise<void>;
  probeEmbeddingAvailability(): Promise<MemoryEmbeddingProbeResult>;
  close?(): Promise<void>;
}
```

### MemorySearchResult

```typescript
export type MemorySearchResult = {
  path: string;        // 文件路径
  startLine: number;  // 起始行
  endLine: number;    // 结束行
  score: number;      // 相关性分数
  snippet: string;    // 文本片段
  source: "memory" | "sessions";
  citation?: string;
};
```

---

## 混合搜索（Hybrid Search）

### 核心算法（hybrid.ts）

```typescript
export async function mergeHybridResults(params: {
  vector: HybridVectorResult[];      // 向量搜索结果
  keyword: HybridKeywordResult[];    // 关键词搜索结果
  vectorWeight: number;              // 向量权重（0.7）
  textWeight: number;                // 关键词权重（0.3）
  mmr?: Partial<MMRConfig>;          // MMR 配置
  temporalDecay?: Partial<TemporalDecayConfig>;  // 时间衰减
}): Promise<HybridResult[]> {
  // 1. 合并结果（按 id）
  const byId = new Map<string, HybridResult>();

  for (const r of params.vector) {
    byId.set(r.id, { ...r, vectorScore: r.vectorScore, textScore: 0 });
  }

  for (const r of params.keyword) {
    const existing = byId.get(r.id);
    if (existing) {
      existing.textScore = r.textScore;
    } else {
      byId.set(r.id, { ...r, vectorScore: 0, textScore: r.textScore });
    }
  }

  // 2. 计算加权分数
  const merged = Array.from(byId.values()).map((entry) => {
    const score = params.vectorWeight * entry.vectorScore + params.textWeight * entry.textScore;
    return { ...entry, score };
  });

  // 3. 时间衰减（可选）
  const decayed = await applyTemporalDecayToHybridResults(merged, params.temporalDecay);

  // 4. 排序
  const sorted = decayed.toSorted((a, b) => b.score - a.score);

  // 5. MMR 重排序（可选）
  if (params.mmr?.enabled) {
    return applyMMRToHybridResults(sorted, params.mmr);
  }

  return sorted;
}
```

### BM25 评分

```typescript
export function bm25RankToScore(rank: number): number {
  const normalized = Number.isFinite(rank) ? Math.max(0, rank) : 999;
  return 1 / (1 + normalized);
}
```

---

## 后处理（Post-Processing）

### 1. 时间衰减（Temporal Decay）

```typescript
export const DEFAULT_TEMPORAL_DECAY_CONFIG: TemporalDecayConfig = {
  enabled: false,
  halfLifeDays: 30,  // 半衰期 30 天
};

// 衰减公式：score × e^(-λ × ageInDays)
// λ = ln(2) / halfLifeDays
```

**效果**：
- 今天：100%
- 7 天前：84%
- 30 天前：50%
- 90 天前：12.5%

**Evergreen 文件不衰减**：
- MEMORY.md
- 非 dated 文件（如 memory/projects.md）

### 2. MMR 重排序（Maximal Marginal Relevance）

```typescript
export const DEFAULT_MMR_CONFIG: MMRConfig = {
  enabled: false,
  lambda: 0.7,  // 0 = 最大多样性，1 = 最大相关性
};

// 公式：λ × relevance − (1−λ) × max_similarity_to_selected
```

**目的**：去除重复结果，确保多样性

---

## 可复用的设计模式

### 1. 混合搜索策略

```typescript
// 向量搜索 + 关键词搜索 + 加权融合
const vectorResults = await searchVector(query, embedding);
const keywordResults = await searchKeyword(query);
const merged = mergeHybridResults({
  vector: vectorResults,
  keyword: keywordResults,
  vectorWeight: 0.7,
  textWeight: 0.3,
});
```

### 2. 时间衰减

```typescript
function applyTemporalDecay(results: Result[], halfLifeDays: number): Result[] {
  const lambda = Math.log(2) / halfLifeDays;

  return results.map(r => {
    const ageInDays = getAgeInDays(r.path);
    const decayFactor = Math.exp(-lambda * ageInDays);
    return { ...r, score: r.score * decayFactor };
  });
}
```

### 3. MMR 重排序

```typescript
function applyMMR(results: Result[], lambda: number, topK: number): Result[] {
  const selected: Result[] = [];
  const remaining = [...results];

  while (selected.length < topK && remaining.length > 0) {
    let bestScore = -Infinity;
    let bestIdx = 0;

    for (let i = 0; i < remaining.length; i++) {
      const relevance = remaining[i].score;
      const diversity = Math.max(
        ...selected.map(s => jaccard(s.snippet, remaining[i].snippet))
      );
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

---

## 与 ZeroClaw 的差异

| 特性 | OpenClaw | ZeroClaw |
|------|----------|----------|
| **语言** | TypeScript | Rust |
| **存储** | Markdown + SQLite | SQLite |
| **搜索接口** | `search()` + `readFile()` | `recall()` + `get()` |
| **MMR** | ✅ | ❌ |
| **时间衰减** | ✅ | ❌ |
| **Session 隔离** | ❌ | ✅ |
| **Citation** | ✅ | ❌ |

---

## Hipocampo 可复用的设计

✅ **混合搜索**（Vector + BM25 + 加权融合）
✅ **时间衰减**（半衰期 30 天）
✅ **MMR 重排序**（多样性）
✅ **MemorySearchResult** 结构
✅ **readFile()** 增量读取

⚠️ **需要增加**：
- Session 隔离
- Category 分类
- 统一接口（Memory trait）

---

**研究完成时间**: 2026-03-04
