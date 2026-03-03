# Hipocampo - 统一记忆后端

> 兼容 OpenClaw 和 ZeroClaw 的通用记忆系统

## 目标

设计并实现一个统一的记忆后端，使得 OpenClaw (TypeScript) 和 ZeroClaw (Rust) 可以共享同一个记忆系统。

## 核心设计

- **统一 API**: 参考 ZeroClaw 的 Memory trait
- **多后端支持**: SQLite、PostgreSQL、Markdown
- **向量搜索**: 支持多种 embedding provider
- **双语言实现**: Rust + TypeScript

## 项目结构

```
hipocampo/
├── core/           # 核心抽象层
├── rust/           # Rust 实现
├── typescript/     # TypeScript 实现
├── docs/           # 设计文档
└── tests/          # 测试
```

## 状态

🚧 项目启动中
