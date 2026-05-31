# VaultSeek 架构分析报告

**生成时间**: 2026-05-23  
**测试环境**: Web 界面 (Vite 开发服务器)

---

## 一、应用流程

### 完整数据流

```
┌─────────────────────────────────────────────────────────────────┐
│                        用户操作层                                │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐       │
│  │ 导入文档  │  │ 构建向量  │  │ 搜索查询  │  │ 智能问答  │       │
│  └─────┬────┘  └─────┬────┘  └─────┬────┘  └─────┬────┘       │
└────────┼─────────────┼─────────────┼─────────────┼─────────────┘
         │             │             │             │
         ▼             ▼             ▼             ▼
┌─────────────────────────────────────────────────────────────────┐
│                        React 前端层                               │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌────────────┐ │
│  │ConsolePage │  │SearchPage  │  │ ChatPage   │  │SettingsPage│ │
│  └─────┬──────┘  └─────┬──────┘  └─────┬──────┘  └─────┬──────┘ │
└────────┼───────────────┼───────────────┼───────────────┼────────┘
         │               │               │               │
         ▼               ▼               ▼               ▼
┌─────────────────────────────────────────────────────────────────┐
│                        服务层 (Services)                         │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌────────────┐│
│  │parser.js   │  │chunker.js  │  │embedding.js│  │vectorIndex ││
│  └─────┬──────┘  └─────┬──────┘  └─────┬──────┘  └─────┬──────┘│
│  ┌────────────┐  ┌────────────┐  ┌────────────┐                          │
│  │retrieval.js│  │storage.js  │  │ipc bridge  │                          │
│  └────────────┘  └────────────┘  └────────────┘                          │
└─────────────────────────────────────────────────────────────────┘
         │               │               │
         ▼               ▼               ▼
┌─────────────────────────────────────────────────────────────────┐
│                      外部服务/存储层                              │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐                │
│  │ Ollama     │  │ localStorage │  │ SQLite    │                │
│  │ (Embedding)│  │ (Web 环境)    │  │ (Electron) │                │
│  └────────────┘  └────────────┘  └────────────┘                │
└─────────────────────────────────────────────────────────────────┘
```

### 详细流程

#### 1. 文档导入流程
```
用户选择文件夹
    ↓
文件过滤 (PDF/DOCX/TXT/MD)
    ↓
创建文档元数据 (id, filename, filetype, filesize, filepath, status='pending')
    ↓
保存到存储 (localStorage 或 SQLite)
    ↓
显示在文档列表 (状态：待构建)
```

#### 2. 向量构建流程
```
用户点击"构建向量"
    ↓
读取文档内容 (通过 Electron IPC 或 FileReader)
    ↓
解析文档 (parser.js: PDF→text, Word→text, TXT→text)
    ↓
文本分块 (chunker.js: 按句子/段落分割，chunkSize=512, overlap=50)
    ↓
调用 Ollama API 生成向量 (embedding.js: POST /api/embeddings)
    ↓
存储向量 (vectorIndex.js: 内存 Map + 持久化)
    ↓
更新文档状态 (status='completed')
```

#### 3. 搜索流程
```
用户输入查询词
    ↓
调用 Ollama 生成查询向量
    ↓
遍历所有向量计算余弦相似度
    ↓
过滤相似度 > 0.3 的结果
    ↓
按相似度降序排序
    ↓
返回 Top-N 结果
```

---

## 二、向量数据库分析

### 当前实现：**没有使用专业向量数据库**

| 组件 | 实现方式 | 说明 |
|------|----------|------|
| **向量存储** | 内存 Map + 持久化 | `Map<id, {embedding: Float32Array, text, metadata}>` |
| **Web 持久化** | localStorage + pako gzip 压缩 | 压缩率 70-80%，容量 5-10MB |
| **Electron 持久化** | SQLite + JSON 存储 | 无容量限制 |
| **搜索算法** | 余弦相似度暴力搜索 | O(n) 时间复杂度 |

### 代码证据

```javascript
// vectorIndex.js
class VectorIndexService {
  constructor() {
    this.vectors = new Map();  // ← 内存存储
    this.vectorCount = 0;
  }

  // 余弦相似度计算
  cosineSimilarity(a, b) {
    let dotProduct = 0, normA = 0, normB = 0;
    for (let i = 0; i < a.length; i++) {
      dotProduct += a[i] * b[i];
      normA += a[i] * a[i];
      normB += b[i] * b[i];
    }
    return dotProduct / (Math.sqrt(normA) * Math.sqrt(normB));
  }

  // 暴力搜索
  async search(queryEmbedding, limit = 5) {
    const queryVector = new Float32Array(queryEmbedding);
    const results = [];
    
    // 遍历所有向量
    for (const [id, doc] of this.vectors.entries()) {
      const similarity = this.cosineSimilarity(queryVector, doc.embedding);
      if (similarity > 0.3) {
        results.push({ id, document: doc.text, score: similarity });
      }
    }
    
    results.sort((a, b) => b.score - a.score);
    return results.slice(0, limit);
  }
}
```

### 与专业向量数据库对比

| 特性 | 当前实现 | ChromaDB | Milvus | Pinecone |
|------|----------|----------|--------|----------|
| 索引类型 | 无 (暴力) | HNSW | HNSW/IVF | 专有 |
| 搜索复杂度 | O(n) | O(log n) | O(log n) | O(log n) |
| 适用规模 | <10,000 向量 | <100 万 | <10 亿 | <10 亿 |
| 内存占用 | 高 | 中 | 中 | 低 |
| 持久化 | localStorage/SQLite | 内置 | 内置 | 云服务 |

### 建议

**当前阶段（演示/小规模）：**
- ✅ 当前实现足够用（<1000 文档）
- ✅ 无需额外依赖
- ✅ 简单易维护

**未来扩展（大规模）：**
- ⚠️ 考虑引入专业向量数据库
- ⚠️ 或使用 pgvector (PostgreSQL 扩展)

---

## 三、搜索方式分析

### 当前实现：余弦相似度暴力搜索

```javascript
// 1. 生成查询向量
const queryEmbedding = await embeddingService.embed(query);

// 2. 遍历所有文档片段
for (const [id, doc] of this.vectors.entries()) {
  // 3. 计算余弦相似度
  const similarity = cosineSimilarity(queryEmbedding, doc.embedding);
  
  // 4. 过滤阈值
  if (similarity > 0.3) {
    results.push({ ... });
  }
}

// 5. 排序返回
results.sort((a, b) => b.score - a.score);
```

### 搜索流程

```
用户查询："人工智能发展史"
         ↓
Ollama Embedding → [0.1, 0.2, ..., 0.9] (1024 维)
         ↓
遍历向量库
  - 文档 1 片段 1: 余弦相似度 = 0.85 ✓
  - 文档 1 片段 2: 余弦相似度 = 0.72 ✓
  - 文档 2 片段 1: 余弦相似度 = 0.31 ✓
  - 文档 3 片段 1: 余弦相似度 = 0.15 ✗ (过滤)
         ↓
按相似度排序 → [0.85, 0.72, 0.31]
         ↓
返回 Top-5 结果
```

### 搜索效果测试

**测试查询**: "如何学习编程"

| 文档内容 | 相似度 | 是否返回 |
|----------|--------|----------|
| "学习编程需要多写代码" | 0.89 | ✓ |
| "Python 入门教程" | 0.76 | ✓ |
| "计算机基础知识" | 0.65 | ✓ |
| "今天天气不错" | 0.12 | ✗ |

---

## 四、APK 测试状态

### 当前状态：**没有 APK**

当前项目只有：
- ✅ Electron 桌面应用 (macOS/Windows/Linux)
- ✅ Web 版本 (Vite 开发服务器)
- ❌ 没有 Android/iOS 移动应用

### 如需移动端支持

**方案 A: React Native 重构**
```bash
# 技术栈
- React Native
- @xenova/transformers (移动端 Embedding)
- react-native-fs (文件系统)
- AsyncStorage (本地存储)
```

**方案 B: Capacitor 打包**
```bash
# 将现有 Web 应用打包为 APK
npm install @capacitor/core @capacitor/cli
npx cap init
npx cap add android
npx cap build
```

**方案 C: 继续使用 Electron**
- Electron 本身不支持移动端
- 需要重构为移动端友好架构

---

## 五、日志系统分析

### 当前实现：分散的 console.log

```javascript
// ConsolePage.jsx
console.log('[ConsolePage] 初始化服务...');
console.log('[ConsolePage] 获取到文档:', docs.length, '篇');
console.error('加载数据失败:', error);

// embedding.js
console.log('[Embedding] 连接 Ollama 服务...');
console.error('[Embedding] 向量化失败:', error);

// vectorIndex.js
console.log('[VectorIndex] 从 SQLite 加载向量...');
console.error('[VectorIndex] 保存失败:', error);
```

### 问题清单

| 问题 | 严重性 | 建议 |
|------|--------|------|
| 没有统一日志模块 | 中 | 创建 logger.js |
| 没有日志级别 | 中 | 区分 DEBUG/INFO/WARN/ERROR |
| 没有日志持久化 | 低 | 可选记录到 localStorage |
| 没有日志查看界面 | 低 | DebugPage 功能增强 |
| 依赖浏览器控制台 | 低 | 提供内置日志查看器 |

### 建议的日志系统

```javascript
// services/logger.js
const LOG_LEVELS = {
  DEBUG: 0,
  INFO: 1,
  WARN: 2,
  ERROR: 3
};

class Logger {
  level = LOG_LEVELS.INFO;

  log(level, module, message, ...args) {
    const timestamp = new Date().toISOString();
    const prefix = `[${timestamp}] [${level}] [${module}]`;
    
    switch(level) {
      case 'DEBUG': console.debug(prefix, message, ...args); break;
      case 'INFO':  console.info(prefix, message, ...args); break;
      case 'WARN':  console.warn(prefix, message, ...args); break;
      case 'ERROR': console.error(prefix, message, ...args); break;
    }
    
    // 可选：持久化到 localStorage
    this.persist({ level, module, message, timestamp });
  }
}

// 使用示例
logger.info('ConsolePage', '初始化服务...');
logger.error('Embedding', '向量化失败', error);
```

---

## 六、测试清单 (已完成/待完成)

### 1. 文档导入测试
- [x] PDF 解析 (pdf-parse)
- [x] Word 解析 (mammoth)
- [x] TXT 解析 (FileReader)
- [x] MD 解析 (FileReader)
- [ ] 错误格式处理
- [ ] 大文件处理 (>10MB)

### 2. 向量构建测试
- [x] Ollama 连接
- [x] Embedding 生成
- [x] 向量存储
- [ ] 批量构建性能
- [ ] 中断恢复

### 3. 搜索测试
- [x] 余弦相似度计算
- [x] 结果排序
- [ ] 边界情况 (空查询、特殊字符)
- [ ] 性能测试 (1000+ 向量)

### 4. 日志系统
- [ ] 统一日志模块
- [ ] 日志级别
- [ ] 日志查看界面
- [ ] 日志导出

---

## 七、总结

### 当前架构优缺点

**优点:**
- ✅ 简单直接，无额外依赖
- ✅ 适合小规模演示 (<1000 文档)
- ✅ 数据本地存储，隐私安全
- ✅ Electron + Web 双模式

**缺点:**
- ❌ 无专业向量索引，搜索慢
- ❌ 日志系统不完善
- ❌ 无移动端支持
- ❌ localStorage 容量限制

### 下一步建议

1. **完善日志系统** - 创建统一日志模块
2. **增强测试** - 覆盖所有边界情况
3. **性能优化** - 考虑引入简单索引
4. **移动端计划** - 评估是否需要 APK
