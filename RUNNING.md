# VaultSeek 运行指南

## 前置条件

### 1. Ollama 必须运行

VaultSeek 依赖 Ollama 进行向量化（Embedding），需要先启动 Ollama 服务：

```bash
# 启动 Ollama（如果未运行）
ollama serve

# 确保有 embedding 模型
ollama list | grep dmeta-embedding
# 如果没有，拉取模型：
# ollama pull shaw/dmeta-embedding-zh
```

### 2. 安装依赖

```bash
npm install
```

## 启动应用

### 方式一：使用 npm 脚本（推荐）

```bash
# 启动 Electron 桌面应用
npm run electron:dev
```

### 方式二：分别启动

```bash
# 1. 启动 Vite 开发服务器
npm run dev

# 2. 新开终端，启动 Electron
npx electron .
```

## 当前状态

- ✅ Ollama 服务运行中 (http://localhost:11434)
- ✅ Embedding 模型：`shaw/dmeta-embedding-zh`
- ✅ Vite 开发服务器：http://localhost:5176
- ✅ Electron 窗口已打开

## 测试步骤

1. **导入文档**
   - 点击"导入文件夹"按钮
   - 选择包含 PDF/Word/TXT 文件的文件夹
   - 文档列表显示，状态为"pending"

2. **构建向量**
   - 点击"构建向量"按钮
   - 等待处理完成（查看控制台日志）
   - 状态变为"completed"

3. **搜索测试**
   - 导航到"搜索"页面
   - 输入关键词
   - 查看搜索结果

4. **智能问答**
   - 导航到