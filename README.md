# VaultSeek

<div align="center">

**VaultSeek** 是一款专为 **Obsidian** 用户和本地知识库深度使用者设计的桌面级 AI 语义搜索引擎。它采用 **Tauri + Rust** 架构，搭载 C5 高性能内存向量引擎，遵循“100% 本地优先、隐私保障”原则，让您的资料一个字都不出本地，却能用一句话精准找回。

**VaultSeek** is a desktop-level AI semantic search engine designed specifically for **Obsidian** users and power users of local knowledge bases. Built with a **Tauri + Rust** architecture and powered by the high-performance C5 in-memory vector engine, it strictly adheres to the "100% Local-First, Privacy-Guaranteed" principle. Not a single word of your data leaves your device, yet you can retrieve exactly what you need with just one sentence.

![Version](https://img.shields.io/badge/version-1.2.0-blue.svg)
![License](https://img.shields.io/badge/license-MIT-green.svg)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS-lightgrey.svg)

</div>

---

## ✨ 核心特性 / Core Features (v1.2.0)

- **🧠 C5 高效向量引擎 (C5 In-Memory Vector Engine)**: 
  - (ZH) 抛弃传统全表扫描，启动即提取向量至内存。千万级向量矩阵点乘毫秒级无感出结果。
  - (EN) Abandons traditional full-table scans. Vectors are loaded into memory on startup, enabling millisecond-level matrix multiplication for instant semantic retrieval.

- **💬 RAG 增强沉浸式对话 (Immersive RAG Chat)**: 
  - (ZH) 纯净的三栏式布局设计。自动拉取本地知识库片段，结合大模型推理，为您生成带有“来源溯源卡片”的严谨回答。
  - (EN) A clean, three-column immersive layout. Automatically fetches local knowledge base snippets and combines them with LLM reasoning to generate rigorous answers complete with "Source Evidence Cards".

- **🎭 专家面具系统 (Expert Persona Masks)**: 
  - (ZH) 针对不同维度的知识库，内置“医学审查、法务合规、系统架构”等不同角色面具，动态切换 AI 认知模式。
  - (EN) Built-in roles like "Medical Reviewer", "Legal Compliance", and "System Architect" to dynamically shift the AI's cognitive framework based on your domain-specific documents.

- **✂️ 语义切割引擎 (Semantic Chunking Engine)**: 
  - (ZH) 智能识别段落和标点符号边界，告别无脑按字数生硬切片，确保 RAG 检索到的每一个片段都具备完整上下文。
  - (EN) Intelligently detects paragraph and punctuation boundaries, ensuring every chunk retrieved by RAG maintains full contextual integrity without arbitrary word-count splits.

- **🚀 Spotlight 级交互 (Spotlight-Level Interaction)**:
  - (ZH) 瞬间呼出，失焦自动隐身，绝不干扰您的工作流。
  - (EN) Instant invocation via global shortcuts. Auto-hides on blur, seamlessly integrating into your workflow.

- **🛡️ 纯净本地运行 (100% Local Processing)**:
  - (ZH) 所有 Embedding 向量化与存储均在本地完成，支持 MD / PDF / DOCX / TXT / XLSX。
  - (EN) All Embedding vectorization and storage occur completely locally. Supports MD, PDF, DOCX, TXT, and XLSX.

---

## 🛠️ 技术架构 / Tech Stack

- **Frontend**: React 18 + Tailwind CSS + Lucide Icons
- **Backend**: Rust (Tauri v2)
- **Embedding Inference**: ONNX Runtime (ort)
- **Embedding Model**: BGE-Small-ZH (Very fast cold start, ~45MB)
- **Storage Layer**: SQLite (rusqlite) + `parking_lot` RwLock + Bincode Serialization

---

## 🚀 快速开始 / Quick Start

### 1. 下载安装 / Download & Install
前往 [Releases](https://github.com/VariableLab/VaultSeek/releases) 页面，根据您的系统下载最新的安装包。  
Go to the [Releases](https://github.com/VariableLab/VaultSeek/releases) page and download the latest installer for your OS.

### 2. 关联文件夹 / Link Folder
首次运行，点击左侧栏的“导入知识库”，选择您的 Obsidian 库或存有文档的目录。  
On first launch, click "Import Knowledge Base" on the left sidebar and select your Obsidian vault or document folder.

### 3. 操作指引 / Usage
- **面具切换 (Switch Persona)**: 在左侧栏下拉菜单中选择。/ Select from the dropdown in the left sidebar.
- **搜索与问答 (Search & Chat)**: 直接在底部输入框输入问题。/ Type your query in the bottom input box.
- **溯源取证 (Evidence Vault)**: 右侧边栏会实时显示本次回答所引用的确凿文档来源。/ The right sidebar will dynamically display the exact document sources used for the response.

---

## 📦 开发者指南 / Developer Guide

如果您想在本地构建或贡献代码 / If you want to build locally or contribute:

```bash
# Clone the repository
git clone https://github.com/VariableLab/VaultSeek.git
cd VaultSeek

# Install dependencies
npm install

# Run development preview
npm run tauri dev

# Build for production
npm run tauri build
```

---

## ⚖️ 开源协议 / License

本项目采用 / This project is licensed under the **MIT License**.

---

> **VaultSeek**: Privacy is productivity. Retrieval is the genesis of knowledge.
> 
> **VaultSeek**：隐私是第一生产力，检索是知识的起点。
