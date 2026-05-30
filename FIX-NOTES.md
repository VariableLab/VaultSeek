# VaultSeek 问题修复指南

## 当前版本与之前版本的主要区别

### 之前版本（localStorage + pako 压缩）

**架构**:
```
浏览器环境
├── localStorage 存储文档元数据
├── localStorage + pako 压缩存储向量
└── 内存 Map 运行时数据

数据流:
导入文档 → 解析 → 分块 → Embedding → 压缩 → localStorage
```

**特点**:
- ✅ 数据存储在浏览器 localStorage
- ✅ 使用 pako gzip 压缩，压缩率 70-80%
- ✅ 刷新页面数据不丢失
- ❌ 受限于 localStorage 容量（5-10MB）
- ❌ 无法跨设备/会话共享

### 当前版本（Electron + SQLite）

**架构**:
```
Electron 应用
├── 主进程 (main.js)
│   ├── SQLite 数据库 (better-sqlite3)
│   │   ├── documents 表
│   │   ├── vectors 表
│   │   ├── user_config 表
│   │   └── usage_stats 表
│   └── IPC 处理器
└── 渲染进程 (React)
    ├── 服务层 (storage.js, vectorIndex.js)
    └── IPC 通信
```

**特点**:
- ✅ 数据存储在 SQLite 数据库
- ✅ 无容量限制
- ✅ 数据持久化，重启不丢失
- ✅ 支持复杂查询
- ⚠️ 需要处理原生模块编译

## 发现的问题及修复

### 问题 1: 向量无法保存到数据库

**症状**: 点击"构建向量"后，所有文档都显示"错误"状态

**原因**: 
1. `addDocuments` 方法中，向量数据格式不一致
2. Electron 的 IPC 传递 Float32Array 会丢失类型信息
3. 保存逻辑中未正确处理数组到 Float32Array 的转换

**修复**:
```javascript
// vectorIndex.js - addDocuments 方法
async addDocuments(documents) {
  for (const doc of documents) {
    const { id, documentId, chunkIndex, chunkText, embedding } = doc;
    
    // 确保 embedding 是普通数组，不是 Float32Array
    this.vectors.set(id, {
      embedding: new Float32Array(embedding), // 重新创建
      text: chunkText,
      metadata: { docId: documentId, chunkIndex },
      createdAt: Date.now(),
    });
  }
  
  // 保存到 SQLite
  await this.saveToStorage();
}
```

### 问题 2: 搜索功能找不到内容

**症状**: 搜索页面输入关键词后，返回空结果

**原因**:
1. 向量数据没有正确保存到 SQLite
2. `saveToStorage` 方法中，向量的 `documentId` 和 `chunkIndex` 字段名不匹配
3. 加载时字段名解析错误

**修复**:
```javascript
// saveToStorage 方法
const vectors = [];
for (const [id, doc] of this.vectors.entries()) {
  vectors.push({
    id,
    documentId: doc.metadata?.docId || doc.metadata?.documentId || '',
    chunkIndex: doc.metadata?.chunkIndex || 0,
    chunkText: doc.text || '',
    embedding: Array.from(doc.embedding), // Float32Array -> 数组
  });
}

if (vectors.length > 0) {
  await window.electronAPI.saveVectors(vectors);
}
```

### 问题 3: 数据持久化路径

**症状**: 重启应用后，之前的数据丢失

**原因**:
1. Electron 主进程的 SQLite 数据库路径正确，但前端服务没有正确加载
2. 初始化顺序问题

**修复**:
```javascript
// ConsolePage.jsx - initServices
useEffect(() => {
  const initServices = async () => {
    // 1. 初始化存储服务
    await storageService.init();
    
    // 2. 初始化向量索引（从 SQLite 加载）
    await vectorIndexService.init();
    
    // 3. 加载数据
    await loadData();
    
    setIsInitialized(true);
  };
  initServices();
}, []);
```

## 使用流程

### 正确的使用方式

1. **启动应用**
   ```bash
   npm run electron:dev
   ```

2. **导入文档**
   - 点击"导入文件夹"按钮
   - 选择包含 PDF/Word/TXT 文件的文件夹
   - 文档列表显示，状态为"pending"

3. **构建向量**
   - 点击"构建向量"按钮
   - 等待处理完成
   - 状态变为"completed"
   - 向量保存到 SQLite

4. **搜索**
   - 导航到"搜索"页面
   - 输入关键词
   - 查看搜索结果

5. **智能问答**
   - 导航到"智能问答"页面
   - 输入问题
   - 获取基于文档的回答

### 数据存储位置

**macOS**:
```
~/Library/Application Support/vaultseek/vaultseek.db
```

**Windows**:
```
%APPDATA%/vaultseek/vaultseek.db
```

**Linux**:
```
~/.config/vaultseek/vaultseek.db
```

## 调试技巧

### 查看控制台日志

打开 Electron 的开发者工具（Ctrl/Cmd + Shift + I），查看 Console 标签页：

```javascript
[ConsolePage] 初始化服务...
[ConsolePage] 服务初始化完成
[ConsolePage] 加载数据...
[ConsolePage] 获取到文档：3 篇
[ConsolePage] 向量数量：15
[VectorIndex] 从 SQLite 加载 15 个向量
```

### 检查 SQLite 数据库

```bash
# macOS
sqlite3 ~/Library/Application\ Support/vaultseek/vaultseek.db "SELECT * FROM documents;"
sqlite3 ~/Library/Application\ Support/vaultseek/vaultseek.db "SELECT COUNT(*) FROM vectors;"
```

### 清空数据

```bash
# 删除数据库文件
rm -rf ~/Library/Application\ Support/vaultseek/vaultseek.db

# 或者在设置页点击"清空全部"
```

## 下一步优化

1. **进度条改进** - 显示真实的处理进度
2. **错误处理优化** - 更友好的错误提示
3. **性能优化** - 大批量文档处理
4. **自动构建** - 导入后自动开始构建
