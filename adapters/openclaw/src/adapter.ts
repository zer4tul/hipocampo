/**
 * Hipocampo adapter for OpenClaw
 *
 * Provides TypeScript API compatible with OpenClaw's MemorySearchManager
 */

import Database from 'better-sqlite3';
import path from 'path';
import fs from 'fs';
import {
  MemoryEntry,
  MemoryCategory,
  SearchOptions,
  ListFilter,
  IndexStats,
  MemorySearchManager,
  HipocampoConfig,
} from './types';

export class HipocampoAdapter implements MemorySearchManager {
  private db: Database.Database;
  private workspace: string;

  constructor(config: HipocampoConfig) {
    this.workspace = config.workspace;
    const dbPath = path.join(config.workspace, 'memory', 'hipocampo.db');

    // Ensure directory exists
    const dbDir = path.dirname(dbPath);
    if (!fs.existsSync(dbDir)) {
      fs.mkdirSync(dbDir, { recursive: true });
    }

    this.db = new Database(dbPath);
    this.db.pragma('journal_mode = WAL');
    this.db.pragma('synchronous = NORMAL');

    this.initSchema();
  }

  private initSchema(): void {
    this.db.exec(`
      CREATE TABLE IF NOT EXISTS memories (
        id          TEXT PRIMARY KEY,
        key         TEXT NOT NULL UNIQUE,
        content     TEXT NOT NULL,
        category    TEXT NOT NULL DEFAULT 'core',
        embedding   BLOB,
        created_at  TEXT NOT NULL,
        updated_at  TEXT NOT NULL,
        session_id  TEXT
      );

      CREATE INDEX IF NOT EXISTS idx_memories_category ON memories(category);
      CREATE INDEX IF NOT EXISTS idx_memories_key ON memories(key);
      CREATE INDEX IF NOT EXISTS idx_memories_session ON memories(session_id);

      CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
        key, content, content=memories, content_rowid=rowid
      );

      CREATE TRIGGER IF NOT EXISTS memories_ai AFTER INSERT ON memories BEGIN
        INSERT INTO memories_fts(rowid, key, content)
        VALUES (new.rowid, new.key, new.content);
      END;

      CREATE TRIGGER IF NOT EXISTS memories_ad AFTER DELETE ON memories BEGIN
        INSERT INTO memories_fts(memories_fts, rowid, key, content)
        VALUES('delete', old.rowid, old.key, old.content);
      END;

      CREATE TRIGGER IF NOT EXISTS memories_au AFTER UPDATE ON memories BEGIN
        INSERT INTO memories_fts(memories_fts, rowid, key, content)
        VALUES('delete', old.rowid, old.key, old.content);
        INSERT INTO memories_fts(rowid, key, content)
        VALUES (new.rowid, new.key, new.content);
      END;
    `);
  }

  async store(
    key: string,
    content: string,
    category: MemoryCategory,
    sessionId?: string
  ): Promise<string> {
    const id = this.generateId();
    const now = new Date().toISOString();

    const stmt = this.db.prepare(`
      INSERT INTO memories (id, key, content, category, created_at, updated_at, session_id)
      VALUES (?, ?, ?, ?, ?, ?, ?)
    `);

    stmt.run(id, key, content, category, now, now, sessionId || null);

    return id;
  }

  async search(query: string, opts?: SearchOptions): Promise<MemoryEntry[]> {
    const limit = opts?.limit || 10;
    const ftsQuery = this.buildFtsQuery(query);

    const stmt = this.db.prepare(`
      SELECT m.id, m.key, m.content, m.category, m.created_at, m.session_id
      FROM memories m
      JOIN memories_fts fts ON m.rowid = fts.rowid
      WHERE memories_fts MATCH ?
      ORDER BY bm25(memories_fts) ASC
      LIMIT ?
    `);

    const rows = stmt.all(ftsQuery, limit) as any[];

    return rows.map((row) => ({
      id: row.id,
      key: row.key,
      content: row.content,
      category: row.category as MemoryCategory,
      timestamp: row.created_at,
      sessionId: row.session_id || undefined,
      score: 0.001, // BM25 score placeholder
    }));
  }

  async get(key: string): Promise<MemoryEntry | null> {
    const stmt = this.db.prepare(`
      SELECT id, key, content, category, created_at, session_id
      FROM memories WHERE key = ?
    `);

    const row = stmt.get(key) as any;

    if (!row) {
      return null;
    }

    return {
      id: row.id,
      key: row.key,
      content: row.content,
      category: row.category as MemoryCategory,
      timestamp: row.created_at,
      sessionId: row.session_id || undefined,
    };
  }

  async list(filter?: ListFilter): Promise<MemoryEntry[]> {
    let sql = 'SELECT id, key, content, category, created_at, session_id FROM memories';
    const conditions: string[] = [];
    const params: any[] = [];

    if (filter?.category) {
      conditions.push('category = ?');
      params.push(filter.category);
    }

    if (filter?.sessionId) {
      conditions.push('session_id = ?');
      params.push(filter.sessionId);
    }

    if (conditions.length > 0) {
      sql += ' WHERE ' + conditions.join(' AND ');
    }

    sql += ' ORDER BY created_at DESC';

    if (filter?.limit) {
      sql += ' LIMIT ?';
      params.push(filter.limit);
    }

    const stmt = this.db.prepare(sql);
    const rows = stmt.all(...params) as any[];

    return rows.map((row) => ({
      id: row.id,
      key: row.key,
      content: row.content,
      category: row.category as MemoryCategory,
      timestamp: row.created_at,
      sessionId: row.session_id || undefined,
    }));
  }

  async forget(key: string): Promise<boolean> {
    const stmt = this.db.prepare('DELETE FROM memories WHERE key = ?');
    const result = stmt.run(key);
    return result.changes > 0;
  }

  async count(): Promise<number> {
    const stmt = this.db.prepare('SELECT COUNT(*) as count FROM memories');
    const row = stmt.get() as any;
    return row.count;
  }

  async healthCheck(): Promise<boolean> {
    try {
      this.db.exec('SELECT 1');
      return true;
    } catch {
      return false;
    }
  }

  /**
   * Index markdown files from workspace
   */
  async index(): Promise<IndexStats> {
    const stats: IndexStats = {
      memoryMd: 0,
      dailyFiles: 0,
      dailyChunks: 0,
      total: await this.count(),
    };

    // Index MEMORY.md
    const memoryMdPath = path.join(this.workspace, 'MEMORY.md');
    if (fs.existsSync(memoryMdPath)) {
      const content = fs.readFileSync(memoryMdPath, 'utf-8');
      const chunks = this.chunkMarkdown(content);
      stats.memoryMd = chunks.length;

      for (const chunk of chunks) {
        const key = `MEMORY.md:${chunk.startLine}:${chunk.endLine}`;
        const existing = await this.get(key);
        if (!existing) {
          await this.store(key, chunk.content, 'core');
        }
      }
    }

    // Index memory/*.md
    const memoryDir = path.join(this.workspace, 'memory');
    if (fs.existsSync(memoryDir)) {
      const files = fs.readdirSync(memoryDir).filter((f) => f.endsWith('.md'));

      for (const file of files) {
        const filePath = path.join(memoryDir, file);
        const content = fs.readFileSync(filePath, 'utf-8');
        const chunks = this.chunkMarkdown(content);

        stats.dailyFiles++;
        stats.dailyChunks += chunks.length;

        for (const chunk of chunks) {
          const key = `memory/${file}:${chunk.startLine}:${chunk.endLine}`;
          const existing = await this.get(key);
          if (!existing) {
            await this.store(key, chunk.content, 'daily');
          }
        }
      }
    }

    stats.total = await this.count();
    return stats;
  }

  /**
   * Close database connection
   */
  close(): void {
    this.db.close();
  }

  private generateId(): string {
    return `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
  }

  private buildFtsQuery(raw: string): string {
    const tokens = raw.split(/\s+/).filter((t) => t.length > 0);
    if (tokens.length === 0) {
      return '""';
    }
    return tokens.map((t) => `"${t.replace(/"/g, '')}"`).join(' AND ');
  }

  private chunkMarkdown(text: string): Array<{ content: string; startLine: number; endLine: number }> {
    const chunks: Array<{ content: string; startLine: number; endLine: number }> = [];
    const lines = text.split('\n');
    let currentChunk: string[] = [];
    let startLine = 1;
    let currentLine = 1;
    const maxChars = 1000;

    for (const line of lines) {
      if (line.startsWith('# ') || line.startsWith('## ') || line.startsWith('### ')) {
        // New section - save current chunk if not empty
        if (currentChunk.length > 0) {
          const content = currentChunk.join('\n');
          chunks.push({
            content,
            startLine,
            endLine: currentLine - 1,
          });
          currentChunk = [];
          startLine = currentLine;
        }
      }

      currentChunk.push(line);

      // Split if too long
      if (currentChunk.join('\n').length > maxChars) {
        const content = currentChunk.join('\n');
        chunks.push({
          content,
          startLine,
          endLine: currentLine,
        });
        currentChunk = [];
        startLine = currentLine + 1;
      }

      currentLine++;
    }

    // Add last chunk
    if (currentChunk.length > 0) {
      const content = currentChunk.join('\n');
      chunks.push({
        content,
        startLine,
        endLine: currentLine - 1,
      });
    }

    return chunks;
  }
}
