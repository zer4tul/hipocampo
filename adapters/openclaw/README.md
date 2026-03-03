# @hipocampo/openclaw-adapter

Hipocampo adapter for OpenClaw - provides TypeScript API compatible with OpenClaw's `MemorySearchManager`.

## Installation

```bash
npm install @hipocampo/openclaw-adapter
```

## Usage

```typescript
import { HipocampoAdapter } from '@hipocampo/openclaw-adapter';

// Initialize
const memory = new HipocampoAdapter({
  workspace: '/path/to/workspace'
});

// Index markdown files
const stats = await memory.index();
console.log(`Indexed ${stats.total} memories`);

// Search
const results = await memory.search('Matrix 配置', { limit: 10 });

// Store
const id = await memory.store('my-key', 'Important fact', 'core');

// Get
const entry = await memory.get('my-key');

// List
const entries = await memory.list({ category: 'core', limit: 100 });

// Forget
await memory.forget('my-key');

// Health check
const healthy = await memory.healthCheck();
```

## API

### `HipocampoAdapter`

#### Constructor

```typescript
new HipocampoAdapter(config: HipocampoConfig)
```

**HipocampoConfig**:
- `workspace: string` - Path to workspace directory
- `embedding?: EmbeddingConfig` - Optional embedding configuration

**EmbeddingConfig**:
- `provider: 'openai' | 'none'`
- `model?: 'text-embedding-3-small' | 'text-embedding-3-large' | 'text-embedding-ada-002'`
- `apiKey?: string`

#### Methods

- `store(key, content, category, sessionId?): Promise<string>`
- `search(query, opts?): Promise<MemoryEntry[]>`
- `get(key): Promise<MemoryEntry | null>`
- `list(filter?): Promise<MemoryEntry[]>`
- `forget(key): Promise<boolean>`
- `count(): Promise<number>`
- `healthCheck(): Promise<boolean>`
- `index(): Promise<IndexStats>`
- `close(): void`

## Compatibility

This adapter implements OpenClaw's `MemorySearchManager` interface, making it a drop-in replacement for OpenClaw's memory system.

## License

MIT
