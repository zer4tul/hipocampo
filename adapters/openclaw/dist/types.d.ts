/**
 * Types compatible with OpenClaw's memory system
 */
export interface MemoryEntry {
    id: string;
    key: string;
    content: string;
    category: MemoryCategory;
    timestamp: string;
    sessionId?: string;
    score?: number;
}
export type MemoryCategory = 'core' | 'daily' | 'conversation' | string;
export interface SearchOptions {
    limit?: number;
    minScore?: number;
    sessionId?: string;
    hybrid?: boolean;
    vectorWeight?: number;
    keywordWeight?: number;
}
export interface ListFilter {
    category?: MemoryCategory;
    sessionId?: string;
    limit?: number;
}
export interface IndexStats {
    memoryMd: number;
    dailyFiles: number;
    dailyChunks: number;
    total: number;
}
export interface MemorySearchManager {
    store(key: string, content: string, category: MemoryCategory, sessionId?: string): Promise<string>;
    search(query: string, opts?: SearchOptions): Promise<MemoryEntry[]>;
    get(key: string): Promise<MemoryEntry | null>;
    list(filter?: ListFilter): Promise<MemoryEntry[]>;
    forget(key: string): Promise<boolean>;
    count(): Promise<number>;
    healthCheck(): Promise<boolean>;
}
export interface HipocampoConfig {
    workspace: string;
    embedding?: EmbeddingConfig;
}
export interface EmbeddingConfig {
    provider: 'openai' | 'none';
    model?: 'text-embedding-3-small' | 'text-embedding-3-large' | 'text-embedding-ada-002';
    apiKey?: string;
}
