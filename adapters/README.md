# Adapters - Compatibility Layer

This directory contains adapters for OpenClaw (TypeScript) and ZeroClaw (Rust).

## Structure

```
adapters/
├── openclaw/          # TypeScript adapter
│   ├── src/
│   │   ├── index.ts
│   │   ├── adapter.ts
│   │   └── types.ts
│   ├── package.json
│   └── tsconfig.json
└── zeroclaw/          # Rust adapter (embedded in main crate)
    └── lib.rs
```

## OpenClaw Adapter

**Purpose**: Provide TypeScript API compatible with OpenClaw's `MemorySearchManager`

**Usage**:
```typescript
import { HipocampoAdapter } from '@hipocampo/openclaw-adapter';

const memory = new HipocampoAdapter({
  workspace: '/path/to/workspace'
});

await memory.index();
const results = await memory.search('query');
```

## ZeroClaw Adapter

**Purpose**: Implement ZeroClaw's `Memory` trait

**Usage**:
```rust
use hipocampo::adapters::zeroclaw::HipocampoBackend;

let backend = HipocampoBackend::new(workspace)?;
let results = backend.search("query", SearchOptions::default()).await?;
```

## Status

- [ ] OpenClaw adapter (TypeScript)
- [ ] ZeroClaw adapter (Rust)
- [ ] Integration tests
