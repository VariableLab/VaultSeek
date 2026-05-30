# VaultSeek Web 界面测试计划

## 应用架构总结

### 1. 向量数据库

**当前实现：内存 Map + localStorage 压缩存储**

```
浏览器环境 (Web)
├──  向量存储：内存 Map (Float32Array)
├──  持久化：localStorage + pako gzip 压缩
└──  压缩率：约 70-80%

Electron 环境
├── 向量存储：SQLite 数据库
│   ├── vectors 表
│   │   - id: TEXT
│   │   - document_id: TEXT
│   │   - chunk_index: INTEGER
│   │   - chunk_text: TEXT
│   │   - embedding: TEXT (JSON 字符串)
│   └── documents 表
└── 内存缓存：Map 结构
```

**问题：没有使用专业向量数据库**
- ❌ 没有使用 ChromaDB、Pinecone、Milvus 等专业向量数据库
- ❌ 搜索使用余弦相似度暴力计算
- ⚠️ 大量向量时性能会下降

### 2. 向量化流程

```
文件导入 → 解析 (parser.js) → 分块 (chunker.js) → Embedding (ollama) → 存储
                                                        ↓
                                               内存 Map + localStorage/SQLite
```

### 3. 搜索方式

**余弦相似度暴力搜索**

```javascript
// vectorIndex.js
async search(queryEmbedding, limit = 5) {
  const queryVector = new Float32Array(queryEmbedding);
  const results = [];
  
  // 遍历所有向量，计算余弦相似度
  for (const [id, doc] of this.vectors.entries()) {
    const similarity = this.cosineSimilarity(queryVector, doc.embedding);
    if (similarity > 0.3) {
      results.push({ id, document: doc.text, metadata: doc.metadata, score: similarity });
    }
  }
  
  // 按相似度排序
  results.sort((a, b) => b.score - a.score);
  return results.slice(0, limit);
}

cosineSimilarity(a, b) {
  let dotProduct = 0, normA = 0, normB = 0;
  for (let i = 0; i < a.length; i++) {
    dotProduct += a[i] * b[i];
    normA += a[i] * a[i];
    normB += b[i] * b[i];
  }
  if (normA === 0 || normB === 0) return 0;
  return dotProduct / (Math.sqrt(normA) * Math.sqrt(normB));
}
```

### 4. 日志系统

**当前实现：console.log 分散在各处**

```javascript
// 没有统一的日志系统
console.log('[ConsolePage] 初始化服务...');
console.error('[Embedding] 向量化失败:', error);
console.warn('[Chunker] 分块失败');
```

**问题：**
- ❌ 没有日志级别（DEBUG, INFO, WARN, ERROR）
- ❌ 没有日志持久化
- ❌ 没有日志查看界面
- ❌ 依赖浏览器控制台

---

## Web 界面测试清单

### 测试环境
- [x] Ollama 服务运行 (http://localhost:11434)
- [x] Embedding 模型：shaw/dmeta-embedding-zh
- [ ] Vite 开发服务器运行
- [ ] 浏览器打开 (Chrome/Safari)

### 1. 文档导入测试

| 测试项 | 预期结果 | 状态 |
|--------|----------|------|
| 导入 PDF 文件 | 成功解析文本 | ⬜ |
| 导入 Word (.docx) | 成功解析文本 | ⬜ |
| 导入 TXT 文件 | 成功读取内容 | ⬜ |
| 导入 MD 文件 | 成功读取内容 | ⬜ |
| 导入不支持格式 | 提示错误 | ⬜ |
| 批量导入 | 全部列出 | ⬜ |
| 重复导入 | 不重复添加 | ⬜ |

### 2. 向量构建测试

| 测试项 | 预期结果 | 状态 |
|--------|----------|------|
| 点击构建按钮 | 开始处理 | ⬜ |
| 进度条显示 | 实时更新 | ⬜ |
| 文档状态变更 | pending → completed | ⬜ |
| 向量保存成功 | 刷新页面不丢失 | ⬜ |
| 错误处理 | 显示错误信息 | ⬜ |
| 中断后续建 | 可继续构建 | ⬜ |

### 3. 搜索功能测试

| 测试项 | 预期结果 | 状态 |
|--------|----------|------|
| 关键词匹配 | 返回相关结果 | ⬜ |
| 语义搜索 | 同义词匹配 | ⬜ |
| 置信度排序 | 高分在前 | ⬜ |
| 无结果处理 | 友好提示 | ⬜ |
| 空向量库 | 提示导入文档 | ⬜ |

### 4. 日志系统测试

| 测试项 | 预期结果 | 状态 |
|--------|----------|------|
| 控制台输出 | 有 [Module] 前缀 | ⬜ |
| 错误日志 | 红色显示 | ⬜ |
| 警告日志 | 黄色显示 | ⬜ |
| 日志级别 | 可过滤 | ⬜ |

### 5. 性能测试

| 测试项 | 预期结果 | 状态 |
|--------|----------|------|
| 10 个文档 | <5 秒 | ⬜ |
| 100 个文档 | <30 秒 | ⬜ |
| 搜索响应 | <1 秒 | ⬜ |
| 内存占用 | <200MB | ⬜ |
| localStorage 使用率 | <80% | ⬜ |

---

## APK 测试状态

**当前没有 APK 版本**

当前只有：
- Electron 桌面应用 (macOS/Windows/Linux)
- Web 版本 (Vite 开发服务器)

如需 Android APK，需要：
1. 使用 React Native 重构
2. 或使用 Capacitor/Cordova 打包

---

## 测试执行

### 启动 Web 界面测试

```bash
# 1. 启动 Vite 开发服务器
npm run dev

# 2. 浏览器打开
# http://localhost:5173
```

### 测试步骤

1. 打开浏览器开发者工具 (F12)
2. 进入 Console 标签页
3. 观察日志输出
4. 执行上述测试清单

### 查看日志

```bash
# 实时日志输出在浏览器控制台
# 过滤日志关键词：
- [ConsolePage]
- [Embedding]
- [VectorIndex]
- [Parser]
- [Chunker]
```
