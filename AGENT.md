# AGENT.md - 行为与开发流程约束

> **状态**: 草案 v0.1
> **最后更新**: 2026-03-04
> **负责人**: Lead Agent (main)

---

## 1. Agent 行为准则

### 1.1 第一优先级：Agent 优先

- **这不是数据库项目**，这是 Agent 服务项目
- **任何设计决策**，必须优先考虑 Agent 的使用体验
- **如有疑问**，在调研和分析阶段找用户澄清
- **禁止**：为了数据库优化而牺牲 Agent 易用性

### 1.2 调研原则

1. **先查文档再提问**
2. **先查文档再提建议**
3. **先查文档再执行**
4. **充分讨论再决策**

### 1.3 质量意识

- **所有代码必须经过 reviewer 审核**
- **所有测试必须全绿通过**
- **禁止绕过测试**
- **禁止降低测试覆盖率要求**

---

## 2. 开发流程

### 2.1 研究阶段（Phase 0）

**目标**：深入理解需求和技术背景

**步骤**：
1. 研究 OpenClaw 记忆系统实现
2. 研究 ZeroClaw 记忆系统实现
3. 研究 memsearch 分层机制
4. 识别可复用的设计模式
5. 记录所有发现到 `research/` 目录

**输出**：
- `research/openclaw-analysis.md`
- `research/zeroclaw-analysis.md`
- `research/memsearch-analysis.md`

### 2.2 设计阶段（Phase 1）

**目标**：设计系统架构和 API

**步骤**：
1. 创建 SPEC.md（功能规范）
2. 创建 AGENT.md（开发流程）
3. 创建 SCOPE.md（硬边界）
4. 设计数据结构
5. 设计 API 接口
6. 评审设计文档

**输出**：
- `SPEC.md`
- `AGENT.md`
- `SCOPE.md`
- `docs/architecture.md`

### 2.3 实现阶段（Phase 2）

**目标**：实现核心功能

**步骤**：
1. 搭建项目骨架（Cargo.toml、目录结构）
2. 实现 Memory trait（核心接口）
3. 实现 SQLite 后端
4. 实现混合搜索
5. 实现分层机制
6. 编写单元测试

**输出**：
- `src/lib.rs`
- `src/memory.rs`
- `src/sqlite_backend.rs`
- `src/search.rs`
- `tests/*`

### 2.4 兼容层实现（Phase 3）

**目标**：实现 OpenClaw 和 ZeroClaw 兼容层

**步骤**：
1. 实现 OpenClaw adapter（TypeScript）
2. 实现 ZeroClaw adapter（Rust）
3. 兼容性测试
4. 文档编写

**输出**：
- `adapters/openclaw/`
- `adapters/zeroclaw/`
- `tests/compatibility/`

### 2.5 测试与优化（Phase 4）

**目标**：确保质量

**步骤**：
1. 完善测试用例（覆盖率 > 80%）
2. 性能测试
3. Bug 修复
4. 代码审查

**输出**：
- 测试报告
- 性能报告
- 审查记录

---

## 3. 代码规范

### 3.1 Rust 代码规范

- 遵循 Rust 官方风格指南（rustfmt）
- 使用 `cargo clippy` 检查
- 文档注释使用 `///` 和 `//!`
- 错误处理使用 `anyhow::Result`

### 3.2 测试规范

- 单元测试：每个模块必须有对应测试
- 集成测试：`tests/` 目录
- 测试命名：`test_<功能>_<场景>`
- 禁止：`#[ignore]` 跳过测试

### 3.3 文档规范

- 每个公共 API 必须有文档注释
- README.md 包含快速开始
- CHANGELOG.md 记录变更历史

---

## 4. 代码审查标准

### 4.1 Reviewer 职责

- 检查代码逻辑正确性
- 检查代码风格一致性
- 检查测试覆盖率
- 检查文档完整性
- 检查性能影响

### 4.2 审查流程

1. 提交 Pull Request
2. Reviewer 审核（至少 1 人）
3. Tester 测试验证
4. Lead Agent 批准
5. 合并到 main 分支

### 4.3 审查标准

- [ ] 代码逻辑正确
- [ ] 测试全绿通过
- [ ] 覆盖率 > 80%
- [ ] 文档完整
- [ ] 性能达标
- [ ] 无安全隐患

---

## 5. 测试纠错流程

### 5.1 Tester 职责

- 运行所有测试
- 验证功能正确性
- 报告 Bug
- 回归测试

### 5.2 Bug 报告格式

```markdown
**Bug 标题**:

**重现步骤**:
1. ...
2. ...

**预期结果**:

**实际结果**:

**环境**:
- OS:
- Rust version:
- SQLite version:

**日志/截图**:
```

### 5.3 修复流程

1. Tester 报告 Bug
2. Developer 修复
3. Reviewer 审查修复
4. Tester 回归测试
5. 关闭 Bug

---

## 6. 工具使用

### 6.1 Codex ACP

- **用途**: 代码阅读、编码任务
- **模型**: `zai/glm-5`（工具调用）、`bailian/qwen-3.5-plus`（推理）
- **触发**: 复杂编码任务、代码分析

### 6.2 Bees 团队协作

- **Dashboard**: http://localhost:3456
- **团队**: hipocampo（7 人）
- **任务跟踪**: 使用 antfarm CLI

---

## 7. 沟通机制

### 7.1 每日同步

- **时间**: 每天开始工作时
- **内容**: 进度更新、问题、计划
- **方式**: 更新 memory/YYYY-MM-DD.md

### 7.2 问题升级

1. **技术问题**: 先查文档 → 搜索代码 → 询问团队
2. **设计问题**: 记录疑问 → 分析影响 → 找用户澄清
3. **进度问题**: 及时沟通 → 调整计划

### 7.3 决策记录

- 重要决策记录到 MEMORY.md
- 设计变更更新 SPEC.md
- 流程变更更新 AGENT.md

---

**下一步**：
- [ ] 搭建项目骨架
- [ ] 分配研究任务
- [ ] 开始代码实现
