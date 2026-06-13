# 向量资产管理规范 (VECTOR_ASSET_SCHEMA.md)

本文件定义了 VaultSeek 如何将物理文件转化为可检索的“知识资产”。

## 1. 资产关系模型 (Asset Schema)
我们将知识库视为一颗知识树，必须保证从检索结果到原文的 **100% 物理映射**。

```mermaid
graph TD
    A[物理文件: .md/.pdf] -->|解析流水线| B(结构化 Markdown)
    B -->|滑动窗口切块| C{切片 (Chunks)}
    C -->|Embedding| D[向量库 (Vector Index)]
    C -->|BM25| E[倒排索引 (Keyword Index)]
    C -->|映射| F(物理偏移量: Start_Offset - End_Offset)
```

## 2. 字段锚定定义
- **ChunkID**: UUID (全局唯一)
- **SourcePath**: 绝对路径 (用于 `obsidian://` 跳转)
- **Provenance**: 片段在原文中的物理偏移量 (Offset)，用于高亮原文。
- **Metadata**: 包含 `file_type`, `tags` (Obsidian Frontmatter), `last_modified`。

## 3. 资产更新原则
- **写时复制 (CoW)**: 任何文件修改，旧索引立即标记为失效，在后台异步重写新索引，绝不删除旧索引直到新索引通过校验，确保搜索永不中断。
