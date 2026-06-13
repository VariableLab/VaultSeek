# VaultSeek UI/UX 设计规范 (Design System v1.0)

本规范定义了 VaultSeek 的所有视觉元素、交互逻辑与布局原则。所有后续的 UI 开发必须严格遵循此文档，确保产品的高级感与一致性。

---

## 1. 核心设计原则 (Design Principles)
- **深邃感 (Dark & Immersive)**：选用深色调，减少长时间使用的眼部疲劳。
- **空间感 (Spatial)**：利用背景深浅差异（主背景 vs 侧边栏）来区分功能区。
- **无感交互 (Invisible UI)**：窗口控制、拖拽逻辑应像原生系统一样丝滑，不应引起用户注意。

## 2. 视觉令牌 (Design Tokens)

### 2.1 调色板 (Color Palette)
- **主画布 (Background)**: `#1e1e1e`
- **侧边栏 (Sidebar)**: `#18181b`
- **卡片/悬浮 (Surface)**: `#222222`
- **交互/强调 (Primary)**: `#3b82f6` (Blue-600)
- **文字 (Text)**:
    - Primary: `#d4d4d4` (Gray-300)
    - Secondary: `#9ca3af` (Gray-400)
    - Muted: `#525252` (Neutral-600)

### 2.2 字体排版 (Typography)
- **Font Family**: Inter, SF Pro, PingFang SC (优先调用系统默认字体栈).
- **字阶**:
    - **H1/H2**: 24px - 32px (Bold/Black), 用于标题与卡片头部.
    - **Body**: 17px, 行高 1.8 (Leading-Relaxed).
    - **UI Elements**: 12px - 14px (Medium), 用于按钮、标签、侧边栏.

## 3. 三栏式布局规范 (Three-Column Layout)

| 列 | 宽度 | 最小限制 | 职责 |
| :--- | :--- | :--- | :--- |
| **Left** | 260px | 220px | 库管理、历史对话、设置 |
| **Middle** | flex-1 | 400px | 会话主区、输入、总结画布 |
| **Right** | 320px | 280px | 环境信息、来源溯源卡片 |

*   **响应式规则**：当窗口总宽度 `< 800px` 时，自动折叠右侧栏；`< 600px` 时，左侧栏改为抽屉式显示。

## 4. 组件交互规范 (Component Patterns)

### 4.1 窗口控件
- **位置**: 左上角 `absolute` 定位。
- **样式**: 红/黄/绿圆点，悬停显示 `×` `-` `+` 字符。
- **响应**: 点击对应 Tauri 窗口 API。

### 4.2 输入与操作
- **交互状态**: `active:scale-95` (轻微缩放响应)。
- **加载状态**: 所有的网络请求 (RAG/LLM) 必须展示 `Loader2` 旋转动画，严禁无反馈操作。

### 4.3 引用卡片
- **样式**: `bg-[#222222]`，带 `border-neutral-800` 的细边框。
- **悬停效果**: `hover:border-neutral-600`，且内部的“打开原文”按钮在非悬停状态下隐藏。

---

## 5. 验收标准 (Acceptance Criteria)
任何 UI 代码合并前，必须满足：
- [ ] 颜色令牌是否直接引用 Tailwind 变量（严禁硬编码 hex）？
- [ ] 不同分辨率下边栏是否被异常压缩？
- [ ] 点击所有“打开链接/原文”的操作是否都有视觉反馈？
- [ ] 黑暗模式下是否有任何刺眼的纯白背景遗留？
