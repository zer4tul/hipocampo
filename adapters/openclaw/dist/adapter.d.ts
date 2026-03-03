/**
 * Hipocampo adapter for OpenClaw
 *
 * Provides TypeScript API compatible with OpenClaw's MemorySearchManager
 */
import { MemoryEntry, MemoryCategory, SearchOptions, ListFilter, IndexStats, MemorySearchManager, HipocampoConfig } from './types';
export declare class HipocampoAdapter implements MemorySearchManager {
    private db;
    private workspace;
    constructor(config: HipocampoConfig);
    private initSchema;
    store(key: string, content: string, category: MemoryCategory, sessionId?: string): Promise<string>;
    search(query: string, opts?: SearchOptions): Promise<MemoryEntry[]>;
    get(key: string): Promise<MemoryEntry | null>;
    list(filter?: ListFilter): Promise<MemoryEntry[]>;
    forget(key: string): Promise<boolean>;
    count(): Promise<number>;
    healthCheck(): Promise<boolean>;
    /**
     * Index markdown files from workspace
     */
    index(): Promise<IndexStats>;
    /**
     * Close database connection
     */
    close(): void;
    private generateId;
    private buildFtsQuery;
    private chunkMarkdown;
}
