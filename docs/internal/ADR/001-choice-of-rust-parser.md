# ADR 001: 采用 Rust 原生解析流水线替代 MarkItDown

**状态**: Accepted
**决策日期**: 2026-06-06

## 背景 (Context)
VaultSeek 需要处理 PDF, DOCX, Excel 等复杂文档。原计划或竞品方案倾向于引入 Python 生态的 `markitdown` 工具，但 Python 解释器环境会导致应用臃肿且无法离线。

## 决策 (Decision)
在 Rust 原生生态内构建文档解析管线：
- DOCX: `docx-rs`
- Excel: `calamine`
- PDF: `pdf-extract`
所有逻辑封装在 `vs-parser` 内部，统一输出为结构化的 Markdown。

## 后果 (Consequences)
- **正面影响**：分发包体积小，启动快，零 Python 依赖，满足隐私要求。
- **负面代价/妥协**：开发成本高，对复杂排版的 PDF 解析效果可能不如基于云端的视觉识别方案，后续需在 `vs-parser` 中迭代优化。
