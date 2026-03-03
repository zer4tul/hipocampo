/**
 * Hipocampo Core - Memory Interface Definitions
 *
 * Unified memory backend for OpenClaw (TypeScript) and ZeroClaw (Rust)
 */

// ============================================================================
// Types
// ============================================================================

/**
 * Memory categories for organization
 */
export type MemoryCategory = 'core' | 'daily' | 'conversation' | string;

/**
 * A single memory entry
 */
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

/**
 * Search options for recall operations
 */
export interface SearchOptions {
  /** Maximum number of results (default: 10) */
  limit?: number;
  /** Minimum relevance score */
  minScore?: number;
  /** Filter by session */
  sessionId?: string;
  /** Enable hybrid search (BM25 + Vector, default: true) */
  hybrid?: boolean;
  /** Vector weight in hybrid search (0.0-1.0, default: 0.7) */
  vectorWeight?: number;
  /** Keyword weight in hybrid search (0.0-1.0, default: 0.3) */
  keywordWeight?: number;
}

/**
 * Filter for list operations
 */
export interface ListFilter {
  category?: MemoryCategory;
  sessionId?: string;
  limit?: number;
}

// ============================================================================
// Memory Interface
// ============================================================================

/**
 * Core memory interface — implement for any persistence backend
 */
export interface Memory {
  /**
   * Backend name
   */
  name(): string;

  /**
   * Store a memory entry
   * @returns The generated entry ID
   */
  store(
    key: string,
    content: string,
    category: MemoryCategory,
    sessionId?: string
  ): Promise<string>;

  /**
   * Search memories with hybrid search
   */
  search(query: string, opts?: SearchOptions): Promise<MemoryEntry[]>;

  /**
   * Get a specific memory by key
   */
  get(key: string): Promise<MemoryEntry | null>;

  /**
   * List memories with optional filters
   */
  list(filter?: ListFilter): Promise<MemoryEntry[]>;

  /**
   * Remove a memory by key
   * @returns true if the memory was found and removed
   */
  forget(key: string): Promise<boolean>;

  /**
   * Count total memories
   */
  count(): Promise<number>;

  /**
   * Health check
   */
  healthCheck(): Promise<boolean>;
}

// ============================================================================
// Embedding Provider Interface
// ============================================================================

/**
 * Embedding provider interface — convert text to vectors
 */
export interface EmbeddingProvider {
  /**
   * Provider name
   */
  name(): string;

  /**
   * Embedding dimensions
   */
  dimensions(): number;

  /**
   * Embed a batch of texts into vectors
   */
  embed(texts: string[]): Promise<number[][]>;

  /**
   * Embed a single text (convenience method)
   */
  embedOne?(text: string): Promise<number[]>;
}
