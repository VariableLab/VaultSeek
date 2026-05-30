# 配置更新说明

**更新日期**: 2026-05-23

---

## 一、应用架构总结

### 1. LLM 问答配置 ✅

| 配置项 | 值 | 说明 |
|--------|-----|------|
| **API 端点** | `https://deepstock.zone.id` | 固定使用此地址 |
| **API Key** | 用户输入 | 在设置页配置，加密存储 |
| **默认模型** | `google/gemma-2-2b-it` | Google Gemma 2 2B 指令模型 |
| **可选模型** | 5 种 | Gemma 2 9B, Qwen2.5, Llama 3.2 |

### 2. 向量化配置 ✅

| 配置项 | 值 | 说明 |
|--------|-----|------|
| **Embedding 服务** | Ollama | `http://localhost:11434` |
| **Embedding 模型** | `shaw/dmeta-embedding-zh` | 中文向量化专用 |
| **向量维度** | 1024 | Float32 数组 |
| **用途** | 文档向量化 + 搜索向量化 | 压缩存储到 SQLite |

### 3. 向量数据库

| 组件 | 实现方式 | 说明 |
|------|----------|------|
| **存储方式** | 内存 Map + SQLite 持久化 | Electron 环境 |
| **压缩方式** | pako gzip 压缩 | 压缩率 ~75% |
| **搜索算法** | 余弦相似度暴力搜索 | O(n) 时间复杂度 |
| **适用规模** | <10,000 向量 | 适合演示和小规模使用 |

---

## 二、完整数据流

```
┌─────────────────────────────────────────────────────────────────┐
│                        用户界面                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐        │
│  │  导入文档     │  │  构建向量     │  │  智能问答     │        │
│  └───────┬──────┘  └───────┬──────┘  └───────┬──────┘        │
└──────────┼─────────────────┼─────────────────┼─────────────────┘
           │                 │                 │
           ▼                 ▼                 ▼
┌─────────────────────────────────────────────────────────────────┐
│                      服务层                                      │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐               │
│  │ parser.js  │  │chunker.js  │  │embedding.js│               │
│  │ (文档解析) │  │ (文本分块) │  │ (Ollama)   │               │
│  └────────────┘  └────────────┘  └────────────┘               │
│                                                              │
│  ┌────────────────┐  ┌────────────────┐                       │
│  │ vectorIndex.js │  │ retrieval.js    │                      │
│  │ (向量存储)     │  │ (搜索服务)      │                      │
│  └────────────────┘  └────────────────┘                      │
└─────────────────────────────────────────────────────────────────┘
           │                 │
           ▼                 ▼
┌─────────────────────────────────────────────────────────────────┐
│                     外部服务/存储层                               │
│  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐   │
│  │ Ollama         │  │ LLM API        │  │ SQLite         │   │
│  │ Embedding      │  │ deepstock.zone │  │ 持久化存储     │   │
│  │ (向量化)       │  │ (智能问答)     │  │                │   │
│  └────────────────┘  └────────────────┘  └────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### 详细流程

#### 1. 文档导入流程
```
用户选择文件夹
    ↓
文件过滤 (PDF/DOCX/TXT/MD)
    ↓
创建文档元数据 (id, filename, filetype, filepath, status='pending')
    ↓
保存到 SQLite (documents 表)
    ↓
显示在文档列表 (状态：待构建)
```

#### 2. 向量构建流程
```
用户点击"构建向量"
    ↓
读取文档内容 (通过 Electron IPC)
    ↓
parser.js: PDF→text, Word→text, TXT→text
    ↓
chunker.js: 按句子/段落分割 (chunkSize=512, overlap=50)
    ↓
embedding.js: 调用 Ollama API 生成 1024 维向量
    ↓
vectorIndex.js: 存储向量到 SQLite (vectors 表)
    ↓
更新文档状态 (status='completed')
```

#### 3. 智能问答流程
```
用户输入问题
    ↓
retrieval.js: 调用 Ollama 生成查询向量
    ↓
vectorIndex.js: 余弦相似度搜索 Top-N 相关片段
    ↓
构建提示词：系统提示 + 文档片段 + 用户问题
    ↓
调用 LLM API (https://deepstock.zone.id/v1/chat/completions)
    ↓
模型：google/gemma-2-2b-it
    ↓
返回答案 + 来源引用
```

---

## 三、修改的文件

### 1. `electron/main.js`
```javascript
// 默认配置更新
const defaultConfigs = {
  llm_endpoint: 'https://deepstock.zone.id',  // ← 新地址
  llm_api_key: '',
  llm_model: 'google/gemma-2-2b-it',          // ← 新模型
  daily_limit: 10,
};
```

### 2. `src/pages/SettingsPage.jsx`
```javascript
// 默认值更新
const [apiConfig, setApiConfig] = useState({
  endpoint: 'https://deepstock.zone.id',
  apiKey: '',
  model: 'google/gemma-2-2b-it',
});

// 模型选择更新
<select value={apiConfig.model}>
  <option value="google/gemma-2-2b-it">Google Gemma 2 2B</option>
  <option value="google/gemma-2-9b-it">Google Gemma 2 9B</option>
  <option value="Qwen/Qwen2.5-Coder-7B-Instruct">Qwen2.5 Coder 7B</option>
  <option value="Qwen/Qwen2.5-72B-Instruct">Qwen2.5 72B</option>
  <option value="meta-llama/Llama-3.2-3B-Instruct">Llama 3.2 3B</option>
</select>
```

### 3. `src/services/embedding.js` (无需修改)
```javascript
// 已使用 Ollama，保持不变
const OLLAMA_BASE_URL = 'http://localhost:11434';
const MODEL_NAME = 'shaw/dmeta-embedding-zh:latest';
```

---

## 四、测试步骤

### 1. 启动 Ollama (必需)
```bash
# 检查 Ollama 是否运行
ollama list

# 如果没有 shaw/dmeta-embedding-zh，拉取模型
ollama pull shaw/dmeta-embedding-zh
```

### 2. 启动应用
```bash
# 方式 1: Electron 桌面应用
npm run electron:dev

# 方式 2: 仅 Web 界面
npm run dev
```

### 3. 配置 API Key
1. 打开设置页
2. 输入 API Key (必填)
3. 端点已默认：`https://deepstock.zone.id`
4. 模型已默认：`google/gemma-2-2b-it`
5. 点击"保存设置"

### 4. 测试流程
1. **导入文档**: 选择文件夹 → 导入
2. **构建向量**: 点击"构建向量" → 等待完成
3. **搜索测试**: 进入搜索页 → 输入关键词
4. **问答测试**: 进入智能问答 → 输入问题

---

## 五、关键 API 调用

### Ollama Embedding (向量化)
```bash
curl http://localhost:11434/api/embeddings \
  -H "Content-Type: application/json" \
  -d '{"model":"shaw/dmeta-embedding-zh","prompt":"测试文本"}'
```

### LLM Chat (智能问答)
```bash
curl https://deepstock.zone.id/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -d '{
    "model": "google/gemma-2-2b-it",
    "messages": [
      {"role": "system", "content": "你是一个智能助手"},
      {"role": "user", "content": "你好"}
    ]
  }'
```

---

## 六、总结

### 更新内容
- ✅ LLM API 端点：固定为 `https://deepstock.zone.id`
- ✅  默认模型：`google/gemma-2-2b-it`
- ✅ API Key：保留用户输入
- ✅ 向量化：继续使用 Ollama

### 下一步
1. 在设置页配置 API Key
2. 导入文档并构建向量
3. 测试搜索和问答功能
