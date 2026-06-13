# VaultSeek

**VaultSeek** 是一款专为 **Obsidian** 用户和本地知识库深度使用者设计的桌面级 AI 语义搜索引擎。它采用 **Tauri + Rust** 架构，遵循“100% 本地优先、隐私保障”原则，让您的资料一个字都不出本地，却能用一句话精准找回。

![Version](https://img.shields.io/badge/version-1.2.0-blue.svg)
![License](https://img.shields.io/badge/license-MIT-green.svg)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS-lightgrey.svg)

---

## ✨ 核心特性 (v1.2.0)

- **🚀 Spotlight 级交互**: 使用 `Alt + S` (macOS 为 `Option + S`) 瞬间呼出，失焦自动隐身，绝不干扰您的工作流。
- **📖 沉浸式双栏阅读**: 采用高级排版审美，18px 黄金字阶与 1.85x 行高，支持在软件内直接进行深度阅读。
- **🧠 混合检索算法**: 融合了 **向量语义搜索** 与 **精准关键词加权**。精准命中文件名或正文的词条会获得显著权重提升，100% 拒绝语义干扰。
- **🔍 实时高亮标注**: 搜索结果在摘要和正文预览中同步高亮，支持正则安全过滤。
- **🔄 自动增量索引**: 集成 `notify` 监听，无需手动重扫。在 Finder 中拖入文件，后台瞬间完成语义理解。
- **🛡️ 纯净本地运行**: 所有 Embedding 向量化与存储均在本地完成，支持 MD / PDF / DOCX / TXT。

---

## 🛠️ 技术架构

- **前端**: React 18 + Tailwind CSS + Lucide Icons
- **后端**: Rust (Tauri v2)
- **推理引擎**: ONNX Runtime (ort)
- **嵌入模型**: BGE-Small-ZH (约 45MB，极速冷启动)
- **存储层**: SQLite (rusqlite) + Bincode 序列化

---

## 🚀 快速开始

### 1. 下载安装
前往 [Releases](https://github.com/VariableLab/VaultSeek/releases) 页面，根据您的系统下载最新的安装包。

### 2. 关联文件夹
首次运行，点击“关联文件夹”，选择您的 Obsidian 库或存有 PDF 的目录。

### 3. 操作指引
- **唤起/隐藏**: `Alt + S`
- **搜索**: 直接输入关键词或语义句子。
- **切换结果**: 使用键盘 `↑` `↓` 键。
- **查看原文**: 选中结果后按 `Enter` 键。
- **管理目录**: 点击侧边栏顶部的 `+` 按钮可更换知识库。

---

## 📦 开发者指南

如果您想在本地构建或贡献代码：

```bash
# 克隆仓库
git clone https://github.com/VariableLab/VaultSeek.git
cd VaultSeek

# 安装依赖
npm install

# 运行开发预览
npm run tauri dev

# 打包生产版本
npm run tauri build
```

---

## ⚖️ 开源协议

本项目采用 **MIT License**。

---

> **VaultSeek**：隐私是第一生产力，检索是知识的起点。
