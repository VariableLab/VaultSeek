<div align="center">
  <img src="src-tauri/icons/128x128.png" alt="VaultSeek Logo" width="128" height="128">
  <h1>VaultSeek RAG 🧠</h1>
  <p><b>Search your local chaos with AI. Fast. Private. Forever.</b></p>
  <p><b>用 AI 穿透本地的混乱。极速、私密、永久免费。</b></p>

  <p>
    <a href="https://github.com/VariableLab/VaultSeek/releases">Download Latest Release</a> · 
    <a href="#-中文介绍">中文介绍</a>
  </p>
</div>

<hr>

## 🌪️ The Problem: The "Where did I save it?" Hell
We all have a digital "junk yard" on our laptops:
- Thousands of Markdown notes.
- Buried PDF manuals and TXT logs.
- Messy folders named `Draft_v1`, `Final_v2`, `FINAL_REALLY_FINAL`.

**Traditional search fails us:** It only matches exact keywords. If you forget the exact filename, the knowledge is practically dead.

## 🚀 The Solution: VaultSeek RAG
VaultSeek is a **Local-First AI Semantic Search Engine** that gives you instant access to your local files based on *meaning*, not just characters.

### 🔥 Core Features:
- **🧠 Semantic Understanding**: Search for "How to manage money" and find `Quant_Strategy.md`. The AI understands the context.
- **🔒 Privacy First**: 100% Offline. Your data never leaves your machine. No cloud, no API keys, no subscription fees.
- **⚡ Blazing Fast**: Powered by Rust & Tauri. Indexes 1000+ documents in seconds, utilizing a highly optimized local SQLite vector cache.
- **🛠️ Hybrid Search Engine**: Combines **BM25 Keyword Matching** + **ONNX Vector Embedding** to ensure you find exactly what you need with zero noise.
- **📚 Multi-Format Support**: Seamlessly parses `.md`, `.txt`, `.pdf`, and `.docx` out of the box.

---

## 📦 Installation
Download the `.dmg` (macOS) or `.exe` (Windows) from the [Releases](https://github.com/VariableLab/VaultSeek/releases) page.

*(Note: On the first run, the app will download a highly optimized ~45MB ONNX AI model to power the semantic engine).*

## 🛠️ For Developers (Build from source)
Want to hack on VaultSeek? It's easy:

1. Clone the repository:
```bash
git clone https://github.com/VariableLab/VaultSeek.git
cd VaultSeek
```

2. Download the required AI model (BGE-Small-zh):
```bash
chmod +x docs/setup_models.sh
./docs/setup_models.sh
```

3. Install dependencies and run:
```bash
npm install
npm run tauri dev
```

---

<h2 id="-中文介绍">🇨🇳 中文介绍</h2>

## 🌪️ 痛点：文件找不着的“数字地狱”
每个人的硬盘都是一个混乱的“知识垃圾场”：
- 几千份散落的 Markdown 笔记。
- 堆积如山的 PDF 说明书、Word 合同和 TXT 日志。
- **传统搜索的无力**：系统自带搜索只能匹配字面。一旦你忘了文件名，珍贵的知识就成了死数据。

## 🚀 方案：VaultSeek RAG
VaultSeek 是一个 **本地优先的 AI 语义搜索引擎**，让你瞬间直达本地知识。你不必整理文件夹，只需“丢”进去，剩下的交给 AI。

### 🔥 核心优势：
- **🧠 语义理解**：搜“怎么理财”，它能帮你翻出《量化策略.md》。AI 懂你的意思，不只是死板地对暗号。
- **🔒 绝对隐私**：100% 离线运行。数据永不出本地，不联网，不需要购买 API Key。
- **⚡ 极致轻量**：Rust + Tauri 驱动底层。采用 SQLite 向量缓存技术，毫秒级唤起，瞬间穿透千份文档。
- **🛠️ 混合引擎**：**关键词硬匹配** + **向量语义匹配**，既准又全，且自动过滤噪音。
- **📚 全格式解析**：原生支持提取 `.md`, `.txt`, `.pdf`, 和 `.docx`。

## 📦 下载与使用
前往 [Releases](https://github.com/VariableLab/VaultSeek/releases) 页面下载 Mac 或 Windows 安装包。
*(注：首次运行时，软件会自动配置一个仅 45MB 的极速本地 AI 引擎)。*

---

## 📬 Feedback & Support
We are building this in public! If you encounter issues or have feature requests, please:
- Open an [Issue](https://github.com/VariableLab/VaultSeek/issues)
- Or email us at: **nxr5875819@gmail.com**

*Created with ❤️ by VariableLab for those who want to reclaim their digital knowledge.*