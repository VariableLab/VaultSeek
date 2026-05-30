# 搜索功能更新说明

**更新日期**: 2026-05-23

---

## 更新内容

参考 `/Users/liuxuran/Github/obsidian-rag` 的实现逻辑，改进了 VaultSeek 的搜索结果显示功能。

### 1. 新增功能

| 功能 | 说明 | 状态 |
|------|------|------|
| **点击展开/折叠** | 点击搜索结果卡片可展开/折叠查看完整内容 | ✅ 完成 |
| **来源显示** | 显示文档来源路径 | ✅ 完成 |
| **相似度提示** | 显示匹配置信度百分比 | ✅ 完成 |
| **高亮选中** | 选中的卡片有高亮边框和阴影 | ✅ 完成 |

### 2. 修改的文件

#### `src/pages/SearchPage.jsx`

**主要变更**:

1. **新增状态管理**:
```javascript
const [selectedResult, setSelectedResult] = useState(null);
const [relatedNotes, setRelatedNotes] = useState([]);
```

2. **点击处理函数**:
```javascript
const handleResultClick = (result, index) => {
  setSelectedResult(selectedResult === index ? null : index);
};
```

3. **收集相关文件**:
```javascript
const seenSources = new Set();
const notes = [];
searchResults.forEach(item => {
  const src = item.metadata?.source || '';
  if (src && !seenSources.has(src)) {
    seenSources.add(src);
    notes.push({ source: src, title: item.metadata?.name || src });
  }
});
setRelatedNotes(notes);
```

4. **可点击的卡片**:
```jsx
<div
  key={index}
  className={`result-card ${isSelected ? 'selected' : ''}`}
  onClick={() => handleResultClick(result, index)}
>
  {/* 卡片内容 */}
  {isSelected && (
    <div className="result-full-content">
      <h4>完整内容</h4>
      <pre>{result.document}</pre>
    </div>
  )}
</div>
```

#### `src/styles/SearchPage.css`

**新增样式**:

1. **卡片交互**:
```css
.result-card {
  cursor: pointer;
  transition: all 0.2s ease;
}

.result-card:hover {
  border-color: var(--text-secondary);
  box-shadow: var(--shadow-sm);
}

.result-card.selected {
  border-color: var(--text-primary);
  box-shadow: var(--shadow);
  background: var(--bg-secondary);
}
```

2. **完整内容区**:
```css
.result-full-content {
  margin-top: 16px;
  padding: 16px;
  background: var(--bg-primary);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  max-height: 400px;
  overflow-y: auto;
}
```

3. **来源显示**:
```css
.result-source {
  font-size: 12px;
  color: var(--text-tertiary);
  margin-top: 8px;
  padding-top: 8px;
  border-top: 1px solid var(--border);
}
```

---

## 使用方式

### 1. 搜索内容
1. 进入搜索页面
2. 输入关键词
3. 点击"搜索"按钮

### 2. 查看结果
- **点击卡片**: 展开/折叠查看完整内容
- **查看来源**: 卡片底部显示文档来源
- **相似度**: 右上角显示匹配度百分比

### 3. 交互效果
- **悬停**: 卡片边框变色 + 轻微阴影
- **选中**: 边框加粗 + 明显阴影 + 背景色变化
- **展开**: 显示完整内容区域（可滚动）

---

## 对比 Obsidian RAG

### Obsidian RAG 实现
```python
# web.py - Gradio 实现
def do_search(query: str):
    # 向量搜索
    vec_results = collection.query(
        query_embeddings=[embedding],
        n_results=20,
    )
    
    # 构建卡片
    cards = ""
    for i in range(len(vec_results["ids"][0])):
        cards += f"""
<details>
<summary>{metadata['source']}</summary>
{content}
</details>
"""
```

### VaultSeek 实现
```jsx
// SearchPage.jsx - React 实现
<div
  className={`result-card ${isSelected ? 'selected' : ''}`}
  onClick={() => handleResultClick(result, index)}
>
  <div className="result-header">...</div>
  <div className="result-content">{result.document}</div>
  {isSelected && (
    <div className="result-full-content">
      <pre>{result.document}</pre>
    </div>
  )}
</div>
```

### 主要差异

| 特性 | Obsidian RAG | VaultSeek |
|------|--------------|-----------|
| 框架 | Gradio | React |
| 展开方式 | `<details>` 标签 | 状态控制 className |
| 样式 | 默认浏览器样式 | 自定义 CSS |
| 选中状态 | 无 | 高亮边框 + 阴影 |

---

## 下一步优化

### 已完成
- [x] 点击展开/折叠功能
- [x] 来源显示
- [x] 选中高亮效果

### 待实现
- [ ] 文件路径点击跳转（需 Electron 支持）
- [ ] 搜索结果分页
- [ ] 关键词高亮
- [ ] 导出搜索结果

---

## 技术细节

### 状态管理
```javascript
// 当前选中的索引
const [selectedResult, setSelectedResult] = useState(null);

// 相关文件列表
const [relatedNotes, setRelatedNotes] = useState([]);
```

### 性能考虑
- 限制显示结果数量（Top 20）
- 内容预览限制（前 300 字符）
- 按需展开，避免 DOM 过大

### 可访问性
- 支持键盘操作（待添加）
- 焦点状态指示（待完善）
- ARIA 标签（待补充）
