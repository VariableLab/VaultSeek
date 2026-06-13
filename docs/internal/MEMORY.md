# 项目记忆索引 (MEMORY.md)

本文件存储关于本项目开发的关键语境与短期上下文，随每次开发会话同步。

## 1. 当前版本信息
- **版本号**: 1.2.0
- **开发分支**: `main`

## 2. 核心架构设计 (详细设计见 `docs/internal/MASTER_BLUEPRINT.md`)
- **双擎混合检索**: (Vector + BM25 + 动态权重重排)
- **文档解析管线**: (Rust 原生解析 Excel/Word/PDF 为结构化 MD)
- **用户画像系统**: (待开发 - 隐式学习 + 查询扩展)

## 3. 已确立工程治理 (见 `docs/internal/ENGINEERING.md`)
- Rust 严禁 `unwrap`。
- 前端必须严格 TypeScript 类型校验。
- 发布前必须通过 20 项硬性检查。

## 4. 当前重点待办 (Next Actions)
- [x] Phase 2: 完善 `vs-parser` 中对复杂 DOCX/Excel 的解析准确率，并测试对比效果。
- [x] Phase 3: 实现基于用户画像的 Query Expansion (查询扩展) 和 交互式反问系统。
- [ ] Phase 4: 实现跨库批量合成与对比报告导出，建立专家 Prompt 市场。

---
*注：请在每次新会话开始时，通过“读取 docs/internal/MEMORY.md”指令恢复语境。*

