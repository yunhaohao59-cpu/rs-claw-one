# RS-Claw V0.3.0 技术方案

> **核心目标：图形界面** | 次要目标：终端美化 | 预留：L4/L5 接口

---

## 一、版本目标与范围

### 1.1 交付物

| 优先级 | 交付物 | 说明 |
|:---|:---|:---|
| P0 | GUI 桌面应用 | Tauri v2 + Web 前端，完整聊天体验 |
| P1 | 终端美化 | 样式化 CLI 输出，彩色分块 |
| P2 | 架构预留 | L4/L5 trait 接口、插件系统骨架 |

### 1.2 不在范围内

- L4 桌面操控的实际实现（仅定义接口）
- L5 浏览器操控的实际实现（仅定义接口）
- macOS 编译支持
- CI/CD 完整配置（仅预留 workflow 模板）

---

## 二、项目结构重构

将现有单 crate 结构改为 workspace，支撑 GUI 和 TUI 两个新二进制：

```
rs-claw-one/
├── Cargo.toml                    # workspace 根
├── src/
│   ├── lib.rs                    # 新增：库入口，暴露公共 API
│   ├── main.rs                   # CLI 入口（现有逻辑）
│   ├── agent/                    # 不变
│   ├── model/                    # 不变
│   ├── gateway/                  # 不变
│   ├── tools/                    # 不变
│   ├── memory/                   # 不变
│   ├── skill/                    # 不变
│   ├── storage/                  # 不变
│   ├── config/                   # 不变
│   ├── context/                  # 不变
│   └── cli/                      # 不变
├── src-tauri/                    # 新增：Tauri 应用壳
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   ├── icons/                    # 应用图标（256x256, 512x512）
│   └── src/
│       └── main.rs               # Tauri 入口 + commands
├── ui/                           # 新增：Web 前端
│   ├── index.html
│   ├── css/
│   │   ├── variables.css         # 设计 token（颜色、字体、间距）
│   │   ├── base.css              # 重置 + 全局样式
│   │   ├── layout.css            # 布局框架
│   │   ├── components.css        # 组件样式
│   │   └── animations.css        # 动画定义
│   ├── js/
│   │   ├── app.js                # 主入口 + 初始化
│   │   ├── chat.js               # 聊天逻辑 + 流式渲染
│   │   ├── sidebar.js            # 侧边栏交互
│   │   ├── settings.js           # 设置面板
│   │   ├── markdown.js           # Markdown 渲染
│   │   └── utils.js              # 工具函数
│   └── assets/
│       └── icons/                # SVG 图标文件
└── README.md
```

**关键变更：**

1. `src/lib.rs` — 将现有 `main.rs` 中的核心逻辑提取为库，暴露 `Agent`、`Config`、`Storage` 等公共类型
2. `src/main.rs` — 简化为 CLI 入口，调用 `rs_claw::` 库 API
3. `src-tauri/src/main.rs` — Tauri 入口，注册 commands，初始化 Agent 共享状态
4. `ui/` — 纯静态前端文件，由 Tauri 内嵌服务

**Cargo.toml workspace 配置：**

```toml
# 根 Cargo.toml
[workspace]
members = [".", "src-tauri"]

[package]
name = "rs-claw"
version = "0.3.0"
edition = "2021"

[lib]
name = "rs_claw"
path = "src/lib.rs"

[[bin]]
name = "rs-claw"
path = "src/main.rs"

# 现有依赖不变...
```

```toml
# src-tauri/Cargo.toml
[package]
name = "rs-claw-gui"
version = "0.3.0"
edition = "2021"

[dependencies]
rs-claw = { path = ".." }
tauri = { version = "2", features = ["devtools"] }
tauri-plugin-shell = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }

[build-dependencies]
tauri-build = { version = "2", features = [] }
```

---

## 三、GUI 技术方案

### 3.1 技术架构

```
┌─────────────────────────────────────────────────────┐
│                   Tauri 窗口                         │
│  ┌────────────────────────────────────────────────┐  │
│  │              Web 前端 (HTML/CSS/JS)             │  │
│  │                                                  │  │
│  │  invoke("send_message") ──────┐                 │  │
│  │  listen("chat-stream")  ◄─────┤                 │  │
│  │  listen("tool-event")   ◄─────┤                 │  │
│  └───────────────────────────────┼─────────────────┘  │
│                                  │                     │
│  ┌───────────────────────────────▼─────────────────┐  │
│  │              Tauri Commands (Rust)               │  │
│  │                                                  │  │
│  │  send_message()    get_sessions()                │  │
│  │  get_memories()    get_skills()                  │  │
│  │  get_config()      update_config()               │  │
│  │  delete_session()  export_session()              │  │
│  └───────────────────────┬─────────────────────────┘  │
│                          │                             │
│  ┌───────────────────────▼─────────────────────────┐  │
│  │              共享 Agent 运行时                    │  │
│  │                                                  │  │
│  │  Agent ←→ Model Layer ←→ Tools ←→ Memory        │  │
│  │                                                  │  │
│  │  Gateway WS 服务器（可选启动，供外部客户端）       │  │
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

**通信方式：**
- 前端 → 后端：Tauri `invoke()` 调用 Rust commands
- 后端 → 前端：Tauri `emit()` 推送流式事件
- 不走 WebSocket，直接 IPC，延迟最低

**Tauri Commands 定义：**

```rust
// src-tauri/src/main.rs

use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::{Manager, Emitter};

struct AppState {
    agent: Arc<Mutex<rs_claw::agent::Agent>>,
    storage: Arc<rs_claw::storage::Storage>,
    config: Arc<tokio::sync::RwLock<rs_claw::config::Config>>,
}

// 发送消息（流式响应通过事件推送）
#[tauri::command]
async fn send_message(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    message: String,
    session_id: Option<String>,
) -> Result<(), String> {
    let mut agent = state.agent.lock().await;
    agent.chat_stream(&message, session_id.as_deref(), |event| {
        app.emit("chat-event", event).ok();
    }).await.map_err(|e| e.to_string())
}

// 获取会话列表
#[tauri::command]
async fn get_sessions(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<rs_claw::storage::SessionInfo>, String> {
    state.storage.list_sessions().map_err(|e| e.to_string())
}

// 获取记忆列表
#[tauri::command]
async fn get_memories(
    state: tauri::State<'_, AppState>,
    query: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<rs_claw::memory::Memory>, String> {
    // ...
}

// 获取技能列表
#[tauri::command]
async fn get_skills(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<rs_claw::skill::Skill>, String> {
    // ...
}

// 获取/更新配置
#[tauri::command]
async fn get_config(state: tauri::State<'_, AppState>) -> Result<rs_claw::config::Config, String> {
    let config = state.config.read().await;
    Ok(config.clone())
}

#[tauri::command]
async fn update_config(
    state: tauri::State<'_, AppState>,
    new_config: rs_claw::config::Config,
) -> Result<(), String> {
    let mut config = state.config.write().await;
    *config = new_config.clone();
    new_config.save().map_err(|e| e.to_string())
}

// 删除会话
#[tauri::command]
async fn delete_session(
    state: tauri::State<'_, AppState>,
    session_id: String,
) -> Result<(), String> {
    state.storage.delete_session(&session_id).map_err(|e| e.to_string())
}

// 新建会话
#[tauri::command]
async fn new_session(
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    state.storage.create_session().map_err(|e| e.to_string())
}
```

**流式事件数据结构：**

```rust
#[derive(Clone, Serialize)]
#[serde(tag = "type")]
enum ChatEvent {
    #[serde(rename = "text_delta")]
    TextDelta { text: String },
    #[serde(rename = "tool_call")]
    ToolCall { id: String, name: String, arguments: String },
    #[serde(rename = "tool_result")]
    ToolResult { id: String, result: String, duration_ms: u64 },
    #[serde(rename = "reasoning")]
    Reasoning { text: String },
    #[serde(rename = "done")]
    Done { total_tokens: Option<u32> },
    #[serde(rename = "error")]
    Error { message: String },
}
```

---

### 3.2 设计规范

#### 3.2.1 设计语言："熔炉（Forge）"

暗色基调 + 琥珀色高光，传达"精密工具"的气质。深色背景模拟锻造炉的暗处，琥珀色点缀如同炉中火花。

#### 3.2.2 颜色系统

所有颜色定义在 `ui/css/variables.css`：

```css
:root {
  /* ===== 背景层级（从深到浅） ===== */
  --bg-base:       #08090C;    /* 最底层背景，窗口底色 */
  --bg-surface:    #0E1018;    /* 主表面，聊天区背景 */
  --bg-elevated:   #161923;    /* 卡片、侧边栏 */
  --bg-overlay:    #1C2030;    /* 弹窗、下拉菜单 */
  --bg-hover:      #232840;    /* 悬停状态 */
  --bg-active:     #2A3050;    /* 按下/选中状态 */

  /* ===== 边框 ===== */
  --border-subtle:  #1E2235;   /* 细分割线 */
  --border-default: #2A2F45;   /* 常规边框 */
  --border-strong:  #3A4060;   /* 强调边框 */

  /* ===== 强调色 ===== */
  --accent-primary:   #E2A53B; /* 主强调色：琥珀金 */
  --accent-primary-hover: #F0B84E;
  --accent-primary-muted: rgba(226, 165, 59, 0.15);
  --accent-secondary: #4ECDC4; /* 次强调色：青绿（工具/技术元素） */
  --accent-secondary-muted: rgba(78, 205, 196, 0.15);
  --accent-purple:    #A78BFA; /* 技能/记忆标识 */
  --accent-purple-muted: rgba(167, 139, 250, 0.12);

  /* ===== 语义色 ===== */
  --color-success:  #34D399;
  --color-warning:  #FBBF24;
  --color-error:    #F87171;
  --color-info:     #60A5FA;

  /* ===== 文字 ===== */
  --text-primary:   #E8ECF1;   /* 主文字 */
  --text-secondary: #8B95A8;   /* 次文字 */
  --text-muted:     #5A6375;   /* 弱文字 */
  --text-inverse:   #08090C;   /* 反色文字（用于亮色按钮上） */
  --text-accent:    #E2A53B;   /* 强调文字 */

  /* ===== 阴影 ===== */
  --shadow-sm:  0 1px 2px rgba(0, 0, 0, 0.3);
  --shadow-md:  0 4px 12px rgba(0, 0, 0, 0.4);
  --shadow-lg:  0 8px 32px rgba(0, 0, 0, 0.5);
  --shadow-glow: 0 0 20px rgba(226, 165, 59, 0.15);  /* 琥珀色辉光 */

  /* ===== 圆角 ===== */
  --radius-sm:   6px;
  --radius-md:   10px;
  --radius-lg:   16px;
  --radius-xl:   20px;
  --radius-full: 9999px;

  /* ===== 间距（8px 基准网格） ===== */
  --space-1:  4px;
  --space-2:  8px;
  --space-3:  12px;
  --space-4:  16px;
  --space-5:  20px;
  --space-6:  24px;
  --space-8:  32px;
  --space-10: 40px;
  --space-12: 48px;
  --space-16: 64px;

  /* ===== 字体 ===== */
  --font-display: 'JetBrains Mono', 'Fira Code', monospace;
  --font-body:    'IBM Plex Sans', 'Noto Sans SC', -apple-system, sans-serif;
  --font-mono:    'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace;

  /* ===== 字号 ===== */
  --text-xs:   11px;
  --text-sm:   13px;
  --text-base: 14px;
  --text-lg:   16px;
  --text-xl:   20px;
  --text-2xl:  24px;
  --text-3xl:  32px;

  /* ===== 动画 ===== */
  --transition-fast:   120ms ease;
  --transition-normal: 200ms ease;
  --transition-slow:   350ms ease-out;

  /* ===== 布局尺寸 ===== */
  --sidebar-width:    260px;
  --header-height:    52px;
  --input-min-height: 56px;
  --titlebar-height:  36px;
}
```

#### 3.2.3 字体加载

在 `ui/index.html` 的 `<head>` 中加载：

```html
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="https://fonts.googleapis.com/css2?family=IBM+Plex+Sans:wght@400;500;600&family=JetBrains+Mono:wght@400;500;700&display=swap" rel="stylesheet">
```

#### 3.2.4 第三方库

```html
<!-- Markdown 渲染 -->
<script src="https://cdn.jsdelivr.net/npm/marked/marked.min.js"></script>
<!-- 代码高亮 -->
<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/highlight.js@11/styles/github-dark-dimmed.min.css">
<script src="https://cdn.jsdelivr.net/npm/highlight.js@11/highlight.min.js"></script>
```

---

### 3.3 窗口与整体布局

#### 3.3.1 窗口配置

```json
// src-tauri/tauri.conf.json
{
  "app": {
    "windows": [{
      "title": "RS-Claw",
      "width": 1280,
      "height": 800,
      "minWidth": 960,
      "minHeight": 600,
      "decorations": false,
      "center": true
    }]
  }
}
```

- **默认尺寸**：1280 x 800 px
- **最小尺寸**：960 x 600 px
- **自定义标题栏**：`decorations: false`，自行绘制窗口控制按钮

#### 3.3.2 整体布局图

```
┌──────────────────────────────────────────────────────────────────┐
│ 自定义标题栏 (高36px, 背景 #0E1018)                              │
│  [🔨 RS-Claw]                    ─  □  ✕                        │
├────────────┬─────────────────────────────────────────────────────┤
│            │                                                     │
│  侧边栏    │  主内容区                                           │
│  宽260px   │                                                     │
│  背景       │  ┌─ 聊天头部 (高52px) ─────────────────────────┐   │
│  #0E1018   │  │ 会话名: "新对话"    模型: deepseek-chat  [⟳] │   │
│            │  └──────────────────────────────────────────────┘   │
│  ┌────────┐│                                                     │
│  │+ 新对话││  ┌─ 消息列表 (可滚动, 占满剩余空间) ────────────┐   │
│  └────────┘│  │                                               │   │
│            │  │  [用户消息 - 右对齐, 琥珀色边框]              │   │
│  会话列表   │  │                                               │   │
│  ┌────────┐│  │  [AI消息 - 左对齐, 全宽]                     │   │
│  │会话 1  ││  │    文本内容...                                │   │
│  │会话 2 ◀││  │    ┌─ 工具调用卡片 ──────────────┐           │   │
│  │会话 3  ││  │    │ 🔧 fs_list  ✅ 0.3s         │           │   │
│  │会话 4  ││  │    │ 展开查看详情...               │           │   │
│  │  ...   ││  │    └────────────────────────────┘           │   │
│  └────────┘│  │                                               │   │
│            │  └───────────────────────────────────────────────┘   │
│  ──────── │                                                     │
│  底部固定  │  ┌─ 输入区 (底部, 最小高56px) ──────────────────┐   │
│  🧠 3 记忆 │  │  [📎] [输入消息....................] [发送➤] │   │
│  ⚡ 5 技能 │  │  Shift+Enter 换行 │ Enter 发送              │   │
│  ⚙ 设置   │  └──────────────────────────────────────────────┘   │
│  v0.3.0   │                                                     │
└────────────┴─────────────────────────────────────────────────────┘
```

**布局实现：**

```css
/* ui/css/layout.css */

* { margin: 0; padding: 0; box-sizing: border-box; }

html, body {
  height: 100%;
  overflow: hidden;
  background: var(--bg-base);
  color: var(--text-primary);
  font-family: var(--font-body);
  font-size: var(--text-base);
  -webkit-font-smoothing: antialiased;
}

/* 整体容器：垂直分为标题栏 + 内容 */
#app {
  display: flex;
  flex-direction: column;
  height: 100vh;
}

/* 自定义标题栏 */
#titlebar {
  height: var(--titlebar-height);
  background: var(--bg-elevated);
  border-bottom: 1px solid var(--border-subtle);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 var(--space-3);
  -webkit-app-region: drag;       /* 允许拖动窗口 */
  user-select: none;
}

#titlebar .window-controls {
  -webkit-app-region: no-drag;    /* 按钮不拖动 */
}

/* 内容区：水平分为侧边栏 + 主区 */
#content {
  display: flex;
  flex: 1;
  overflow: hidden;
}

/* 侧边栏 */
#sidebar {
  width: var(--sidebar-width);
  min-width: var(--sidebar-width);
  background: var(--bg-elevated);
  border-right: 1px solid var(--border-subtle);
  display: flex;
  flex-direction: column;
}

/* 主内容区 */
#main {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;    /* 防止 flex 子元素溢出 */
  background: var(--bg-surface);
}
```

---

### 3.4 自定义标题栏

```
┌──────────────────────────────────────────────────────────────────┐
│  [🔨图标] RS-Claw                        ─   □   ✕             │
│  12px图标  文字13px                       窗口控制按钮           │
└──────────────────────────────────────────────────────────────────┘
```

**详细规格：**
- 高度：36px
- 背景色：`#0E1018`（`--bg-elevated`）
- 底部边框：1px solid `#1E2235`（`--border-subtle`）
- 左侧：锤子 SVG 图标（16x16px）+ "RS-Claw" 文字（JetBrains Mono, 13px, 500 weight, `--text-secondary`）
- 右侧：三个窗口控制按钮
  - 最小化 `─`：hover 背景 `--bg-hover`
  - 最大化 `□`：hover 背景 `--bg-hover`
  - 关闭 `✕`：hover 背景 `#F87171`，hover 文字白色
  - 按钮尺寸：36x36px，文字 14px，`--text-muted`
- 整个标题栏可拖动（`-webkit-app-region: drag`）
- 按钮区域不可拖动（`-webkit-app-region: no-drag`）

**HTML 结构：**

```html
<div id="titlebar">
  <div class="titlebar-left">
    <svg class="app-icon"><!-- 锤子/钳子图标 SVG --></svg>
    <span class="app-name">RS-Claw</span>
  </div>
  <div class="window-controls">
    <button class="win-btn" id="btn-minimize">─</button>
    <button class="win-btn" id="btn-maximize">□</button>
    <button class="win-btn win-close" id="btn-close">✕</button>
  </div>
</div>
```

---

### 3.5 侧边栏

```
┌──────────────────────┐
│                      │
│  [＋ 新对话]          │  ← 全宽按钮，高40px
│                      │
│  ── 最近会话 ──────  │  ← 分组标题
│                      │
│  ▎今天               │
│  ┌──────────────────┐│
│  │ 📄 帮我列出桌面   ││  ← 选中态：背景 --bg-active
│  │    10:30         ││     左边框 3px --accent-primary
│  └──────────────────┘│
│  ┌──────────────────┐│
│  │ 📄 分析项目结构   ││  ← 普通态：背景 transparent
│  │    09:15         ││     hover: --bg-hover
│  └──────────────────┘│
│                      │
│  ▎昨天               │
│  ┌──────────────────┐│
│  │ 📄 写一个爬虫     ││
│  │    昨天 18:22     ││
│  └──────────────────┘│
│  ┌──────────────────┐│
│  │ 📄 部署到服务器   ││
│  │    昨天 14:00     ││
│  └──────────────────┘│
│                      │
│  (更多会话...)       │
│                      │
│                      │
│  ══════════════════  │  ← 底部分隔线
│                      │
│  🧠 3 条记忆         │  ← 统计信息
│  ⚡ 5 个技能         │
│                      │
│  ⚙ 设置              │  ← 点击打开设置面板
│                      │
│  v0.3.0              │  ← 版本号，--text-muted
└──────────────────────┘
```

**详细规格：**

- 宽度：260px，固定不收缩
- 背景：`--bg-elevated`（#0E1018）
- 右边框：1px solid `--border-subtle`

**新对话按钮：**
- 高度：40px
- 距侧边栏内边距：12px（左右上下）
- 背景：transparent，边框 1px dashed `--border-default`
- 文字："＋ 新对话"，居中，14px，500 weight，`--text-secondary`
- 圆角：`--radius-md`（10px）
- hover：背景 `--accent-primary-muted`，边框变 solid，颜色变 `--accent-primary`，文字变 `--accent-primary`
- transition：`--transition-normal`

**会话分组标题：**
- "今天"、"昨天"、"更早" 等时间分组
- 文字：11px，500 weight，大写，`--text-muted`
- 字间距：0.05em
- 上下 padding：8px（与列表项间距）

**会话列表项：**
- 高度：52px
- padding：8px 12px
- 圆角：`--radius-sm`（6px）
- margin：2px 8px（项间距）
- 布局：左侧 📄 图标（16px，`--text-muted`）+ 右侧文字区
- 标题：13px，500 weight，`--text-primary`，单行截断（`text-overflow: ellipsis`）
- 时间戳：11px，`--text-muted`
- hover：背景 `--bg-hover`
- 选中态：背景 `--bg-active`，左侧 3px solid border `--accent-primary`

**底部统计区：**
- padding：12px
- 分隔线：1px solid `--border-subtle`
- 统计项：图标 + 文字，12px，`--text-muted`
  - "🧠 3 条记忆"
  - "⚡ 5 个技能"
- 设置按钮：40px 高，全宽，hover 背景 `--bg-hover`，图标 ⚙ + "设置" 文字
- 版本号：11px，居中，`--text-muted`

**会话列表按时间分组逻辑：**
- 今天（当日 00:00 之后）
- 昨天
- 本周（昨天之后、本周一之前）
- 更早

---

### 3.6 聊天区头部

```
┌──────────────────────────────────────────────────────┐
│  帮我列出桌面文件          ● deepseek-chat    [⟳] [⋯] │
│  会话标题(16px,600)      模型指示器(12px)    操作按钮  │
└──────────────────────────────────────────────────────┘
```

**详细规格：**
- 高度：52px
- 背景：`--bg-surface`（#0E1018）
- 底部边框：1px solid `--border-subtle`
- padding：0 24px
- 布局：flex，space-between，垂直居中

**左侧 — 会话标题：**
- 字体：16px，600 weight，`--text-primary`
- 最大宽度：50%（防止挤压右侧）
- 溢出截断

**中间 — 模型指示器：**
- 小圆点（8x8px，`--color-success`，表示连接正常）+ 模型名
- 字体：12px，`--text-muted`
- 字体：JetBrains Mono

**右侧 — 操作按钮：**
- 刷新按钮 [⟳]：重新生成，24x24px，hover 旋转 180°
- 更多按钮 [⋯]：下拉菜单（清空对话、导出、查看工具日志）
- 按钮样式：transparent 背景，`--text-muted`，hover `--text-primary`

---

### 3.7 消息列表区域

消息列表占据聊天区除头部和输入区外的所有空间，可垂直滚动。

**滚动条样式：**

```css
#message-list::-webkit-scrollbar {
  width: 6px;
}
#message-list::-webkit-scrollbar-track {
  background: transparent;
}
#message-list::-webkit-scrollbar-thumb {
  background: var(--border-default);
  border-radius: 3px;
}
#message-list::-webkit-scrollbar-thumb:hover {
  background: var(--border-strong);
}
```

**消息间距：**
- 相邻消息间距：16px
- 用户消息与 AI 消息间距：20px（更明显区分）
- 工具调用卡片与上下内容间距：12px

**消息区域内边距：**
- 左右 padding：24px
- 上 padding：20px
- 下 padding：20px
- 消息最大宽度：800px（防止在宽屏上拉伸过长）

---

### 3.8 消息组件设计

#### 3.8.1 用户消息

```
                                          ┌─────────────────────────┐
                                          │ 帮我列出桌面有哪些文件    │
                                          │                         │
                                          │             10:30 ✓✓    │
                                          └─────────────────────────┘
                                          ▲ 右对齐，最大宽度 70%
```

**样式：**
- 对齐：右对齐
- 最大宽度：消息列表宽度的 70%
- 背景：`--bg-active`（#2A3050）
- 边框：无
- 圆角：`--radius-lg`（16px），右上角 `--radius-sm`（6px）—— 气泡效果
- padding：12px 16px
- 文字：14px，`--text-primary`，行高 1.6
- 时间戳：11px，`--text-muted`，右对齐，与正文间距 4px
- 发送状态：✓✓（已读）/ ✓（已发送），颜色 `--text-muted`

**CSS：**

```css
.msg-user {
  align-self: flex-end;
  max-width: 70%;
  background: var(--bg-active);
  border-radius: var(--radius-lg) var(--radius-sm) var(--radius-sm) var(--radius-lg);
  padding: var(--space-3) var(--space-4);
}

.msg-user .msg-text {
  font-size: var(--text-base);
  line-height: 1.6;
  color: var(--text-primary);
  word-break: break-word;
}

.msg-user .msg-meta {
  font-size: var(--text-xs);
  color: var(--text-muted);
  text-align: right;
  margin-top: var(--space-1);
}
```

#### 3.8.2 AI 消息

```
┌──────────────────────────────────────────────────────────────┐
│  你的桌面有以下文件：                                          │
│                                                              │
│  ```bash                                                     │
│  document.pdf   photo.jpg   project/                         │
│  notes.txt      music.mp3   ...                              │
│  ```                                                         │
│                                                              │
│  共 12 个文件和 3 个文件夹。                                  │
│                                                              │
│  ┌─ 🔧 工具调用: fs_list ─────────── ✅ 0.3s ──────────┐   │
│  │  ▸ 参数                                                │   │
│  │  ▸ 结果                                                │   │
│  └───────────────────────────────────────────────────────┘   │
│                                                              │
│                                          10:31    [复制] [⟳] │
└──────────────────────────────────────────────────────────────┘
```

**样式：**
- 对齐：左对齐
- 最大宽度：100%（全宽，但内容区有 800px max-width）
- 背景：`--bg-elevated`（#161923）
- 边框：1px solid `--border-subtle`
- 圆角：`--radius-sm` `--radius-lg` `--radius-lg` `--radius-lg`（左上角小圆角，气泡效果）
- padding：16px 20px
- 文字：14px，`--text-primary`，行高 1.7
- 头像：无（不使用头像，用圆角方向区分）

**Markdown 渲染样式：**

```css
.msg-ai .msg-text h1, .msg-ai .msg-text h2, .msg-ai .msg-text h3 {
  font-family: var(--font-display);
  color: var(--text-primary);
  margin-top: var(--space-4);
  margin-bottom: var(--space-2);
}

.msg-ai .msg-text h1 { font-size: var(--text-xl); }
.msg-ai .msg-text h2 { font-size: var(--text-lg); }
.msg-ai .msg-text h3 { font-size: var(--text-base); font-weight: 600; }

.msg-ai .msg-text p {
  margin-bottom: var(--space-3);
}

.msg-ai .msg-text ul, .msg-ai .msg-text ol {
  padding-left: var(--space-6);
  margin-bottom: var(--space-3);
}

.msg-ai .msg-text li {
  margin-bottom: var(--space-1);
}

.msg-ai .msg-text strong {
  color: var(--accent-primary);
  font-weight: 600;
}

.msg-ai .msg-text a {
  color: var(--accent-secondary);
  text-decoration: none;
  border-bottom: 1px solid transparent;
  transition: border-color var(--transition-fast);
}
.msg-ai .msg-text a:hover {
  border-bottom-color: var(--accent-secondary);
}

/* 行内代码 */
.msg-ai .msg-text code:not(pre code) {
  background: var(--bg-active);
  color: var(--accent-primary);
  padding: 2px 6px;
  border-radius: 4px;
  font-family: var(--font-mono);
  font-size: 0.9em;
}

/* 代码块 */
.msg-ai .msg-text pre {
  background: var(--bg-base);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  padding: var(--space-4);
  margin: var(--space-3) 0;
  overflow-x: auto;
  position: relative;
}

.msg-ai .msg-text pre code {
  font-family: var(--font-mono);
  font-size: var(--text-sm);
  line-height: 1.5;
  color: var(--text-primary);
}

/* 代码块右上角语言标签 */
.msg-ai .msg-text pre::after {
  content: attr(data-lang);
  position: absolute;
  top: 8px;
  right: 12px;
  font-size: 10px;
  color: var(--text-muted);
  font-family: var(--font-mono);
  text-transform: uppercase;
}

/* 代码块复制按钮 */
.code-copy-btn {
  position: absolute;
  top: 8px;
  right: 40px;
  background: var(--bg-hover);
  border: 1px solid var(--border-default);
  color: var(--text-muted);
  padding: 4px 8px;
  border-radius: 4px;
  font-size: 11px;
  cursor: pointer;
  opacity: 0;
  transition: opacity var(--transition-fast);
}
pre:hover .code-copy-btn {
  opacity: 1;
}
```

**表格样式：**

```css
.msg-ai .msg-text table {
  width: 100%;
  border-collapse: collapse;
  margin: var(--space-3) 0;
  font-size: var(--text-sm);
}

.msg-ai .msg-text th {
  background: var(--bg-base);
  padding: var(--space-2) var(--space-3);
  text-align: left;
  font-weight: 600;
  color: var(--text-secondary);
  border-bottom: 2px solid var(--border-default);
}

.msg-ai .msg-text td {
  padding: var(--space-2) var(--space-3);
  border-bottom: 1px solid var(--border-subtle);
}

.msg-ai .msg-text tr:hover td {
  background: var(--bg-hover);
}
```

**消息底部操作栏：**
- 时间戳：左对齐（AI 消息不显示时间戳在右侧，而是底部左）
- 操作按钮：复制 [复制]、重新生成 [⟳]
- 字体：11px，`--text-muted`
- 按钮 hover：`--text-secondary`

#### 3.8.3 系统消息

```
         ────── 会话已恢复，共 12 条历史消息 ──────
```

**样式：**
- 居中
- 文字：12px，`--text-muted`
- 前后横线：1px solid `--border-subtle`，横线与文字间距 12px
- 上下 margin：16px

---

### 3.9 工具调用卡片

工具调用在 AI 消息内部显示为可展开卡片：

**收起态：**

```
┌─ 🔧 fs_list ──────────────────── ✅ 完成 · 0.3s ── ▶ ┐
└────────────────────────────────────────────────────────┘
```

- 高度：40px
- 背景：`--bg-base`（#08090C）
- 边框：1px solid `--border-default`
- 圆角：`--radius-sm`
- 左侧：工具图标（🔧/📁/💻/🌐，14px）+ 工具名（JetBrains Mono，13px，`--accent-secondary`）
- 右侧：状态（✅/⏳/❌）+ 耗时 + 展开箭头（▶）
- hover：边框颜色变 `--accent-secondary`
- 点击展开/收起

**展开态：**

```
┌─ 🔧 fs_list ──────────────────── ✅ 完成 · 0.3s ── ▼ ┐
│                                                         │
│  参数                                                    │
│  ┌──────────────────────────────────────────────────┐  │
│  │ {                                                 │  │
│  │   "path": "/home/user/Desktop"                    │  │
│  │ }                                                 │  │
│  └──────────────────────────────────────────────────┘  │
│                                                         │
│  结果                                                    │
│  ┌──────────────────────────────────────────────────┐  │
│  │ document.pdf                                      │  │
│  │ photo.jpg                                         │  │
│  │ project/                                          │  │
│  │ notes.txt                                         │  │
│  │ music.mp3                                         │  │
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

- 展开后 padding：12px 16px
- "参数"/"结果" 标签：11px，500 weight，`--text-muted`，大写
- 参数/结果代码块：背景 `--bg-surface`，padding 12px，圆角 `--radius-sm`，字体 JetBrains Mono 12px，`--text-secondary`
- JSON 自动格式化，语法高亮
- 最大高度：300px，超出滚动

**执行中态：**

```
┌─ 🔧 shell ─────────────────────── ⏳ 执行中... ─── ▶ ┐
└────────────────────────────────────────────────────────┘
```

- 状态图标：⏳，带旋转动画
- 文字："执行中..."，`--color-warning`
- 边框：左侧 3px solid `--color-warning`

**错误态：**

```
┌─ 🔧 shell ─────────────────────── ❌ 失败 ──────── ▶ ┐
└────────────────────────────────────────────────────────┘
```

- 状态图标：❌
- 文字："失败"，`--color-error`
- 边框：左侧 3px solid `--color-error`

---

### 3.10 输入区

```
┌──────────────────────────────────────────────────────────┐
│  [📎] [输入消息，Shift+Enter 换行................] [➤发送] │
│                                                          │
│  模型: deepseek-chat │ 上下文: 1,234 tokens │ ● 已连接   │
└──────────────────────────────────────────────────────────┘
```

**详细规格：**
- 最小高度：56px（单行时）
- 最大高度：200px（多行时自动扩展）
- 背景：`--bg-elevated`（#161923）
- 顶部边框：1px solid `--border-subtle`
- padding：12px 20px

**输入框容器：**
- 布局：flex，行，垂直居中
- 背景：`--bg-base`（#08090C）
- 边框：1px solid `--border-default`
- 圆角：`--radius-lg`（16px）
- padding：10px 16px
- 聚焦态：边框颜色变 `--accent-primary`，带 `--shadow-glow`

**文本输入区域：**
- 使用 `<textarea>` 元素
- 无边框、无背景（继承容器）
- 字体：14px，`--font-body`，`--text-primary`
- placeholder：14px，`--text-muted`
- 行高：1.5
- 自动扩展高度（JS 监听 input 事件，调整 height）
- 最大高度 200px 后出现滚动条

**发送按钮：**
- 尺寸：36x36px
- 背景：`--accent-primary`（#E2A53B）
- 图标：箭头 ↑ 或 ➤，18px，`--text-inverse`
- 圆角：50%（圆形）
- hover：背景 `--accent-primary-hover`，带 `--shadow-glow`
- disabled 态（输入为空）：背景 `--bg-hover`，图标 `--text-muted`
- transition：`--transition-normal`

**附件按钮：**
- 尺寸：32x32px
- 图标：📎，16px，`--text-muted`
- transparent 背景
- hover：`--text-secondary`
- V0.3.0 暂不实现附件功能，仅放 UI 占位

**状态栏（输入区下方）：**
- 高度：24px
- 背景：`--bg-elevated`
- padding：0 20px
- 左侧：模型名（11px，JetBrains Mono，`--text-muted`）
- 中间：上下文 token 数（11px，`--text-muted`）
- 右侧：连接状态指示灯（8px 圆点 + 文字）
  - 已连接：`--color-success` + "已连接"
  - 连接中：`--color-warning` + "连接中..."（闪烁）
  - 断开：`--color-error` + "已断开"

**键盘交互：**
- Enter：发送消息
- Shift+Enter：换行
- 输入框聚焦时，整个应用的快捷键不冲突

---

### 3.11 空状态（无会话/新对话）

当用户点击"新对话"或首次打开应用时：

```
┌──────────────────────────────────────────────────────────┐
│                                                          │
│                                                          │
│                                                          │
│                      🔨 RS-Claw                          │
│                                                          │
│              Rust 重写的跨平台 AI 电脑助手                 │
│              告诉我你想做什么，我来帮你操作电脑             │
│                                                          │
│                                                          │
│    ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │
│    │   📁          │  │   💻          │  │   🌐          │ │
│    │  文件管理     │  │  命令执行     │  │  网络请求     │ │
│    │  读写文件     │  │  Shell 命令   │  │  HTTP 请求    │ │
│    │  浏览目录     │  │  系统操作     │  │  API 调用     │ │
│    └──────────────┘  └──────────────┘  └──────────────┘ │
│                                                          │
│    ┌──────────────────────────────────────────────────┐ │
│    │ 💡 试试说："帮我查看当前目录有哪些文件"            │ │
│    └──────────────────────────────────────────────────┘ │
│                                                          │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

**详细规格：**
- 垂直水平居中
- 最大宽度：600px

**Logo 区域：**
- 锤子图标：64x64px，`--accent-primary`
- "RS-Claw"：32px，JetBrains Mono，700 weight，`--text-primary`
- 副标题：14px，`--text-secondary`，居中
- 标语：13px，`--text-muted`，居中

**能力卡片（3 张）：**
- 布局：flex，等宽，间距 16px
- 每张卡片：
  - 背景：`--bg-elevated`
  - 边框：1px solid `--border-subtle`
  - 圆角：`--radius-md`
  - padding：24px
  - 图标：32px，居中
    - 文件管理：`--accent-primary`
    - 命令执行：`--accent-secondary`
    - 网络请求：`--accent-purple`
  - 标题：14px，600 weight，`--text-primary`，居中
  - 描述：12px，`--text-muted`，居中
  - hover：边框颜色变对应强调色，translateY(-2px)，`--shadow-md`
  - transition：`--transition-normal`

**提示语：**
- 背景：`--accent-primary-muted`
- 边框：1px solid `rgba(226, 165, 59, 0.2)`
- 圆角：`--radius-md`
- padding：12px 20px
- 文字：13px，`--text-accent`
- 💡 图标在文字左侧

**入场动画：**
- Logo：fadeUp，0.5s，delay 0s
- 能力卡片：fadeUp，0.5s，delay 依次 0.1s、0.2s、0.3s
- 提示语：fadeUp，0.5s，delay 0.5s

---

### 3.12 设置面板

设置以模态弹窗（Overlay）形式展示，覆盖在主界面上：

```
┌────────────────────────────────────────────────────────┐
│                   (半透明遮罩层)                         │
│   ┌────────────────────────────────────────────────┐   │
│   │  设置                                    [✕]   │   │
│   │  ───────────────────────────────────────────── │   │
│   │                                                │   │
│   │  ┌─ 导航 ──┐  ┌─ 内容 ─────────────────────┐  │   │
│   │  │         │  │                              │  │   │
│   │  │ ● API   │  │  API 配置                    │  │   │
│   │  │   配置  │  │                              │  │   │
│   │  │         │  │  Provider                    │  │   │
│   │  │ ○ 模型  │  │  ┌──────────────────────┐   │  │   │
│   │  │   设置  │  │  │ deepseek             │   │  │   │
│   │  │         │  │  └──────────────────────┘   │  │   │
│   │  │ ○ 记忆  │  │                              │  │   │
│   │  │   设置  │  │  API Key                     │  │   │
│   │  │         │  │  ┌──────────────────────┐   │  │   │
│   │  │ ○ 关于  │  │  │ sk-••••••••••••      │   │  │   │
│   │  │         │  │  └──────────────────────┘   │  │   │
│   │  │         │  │                              │  │   │
│   │  │         │  │  Base URL (可选)              │  │   │
│   │  │         │  │  ┌──────────────────────┐   │  │   │
│   │  │         │  │  │ (留空使用默认)        │   │  │   │
│   │  │         │  │  └──────────────────────┘   │  │   │
│   │  │         │  │                              │  │   │
│   │  │         │  │            [测试连接] [保存]  │  │   │
│   │  └─────────┘  └──────────────────────────────┘  │   │
│   └────────────────────────────────────────────────┘   │
└────────────────────────────────────────────────────────┘
```

**弹窗规格：**
- 遮罩层：背景 `rgba(0, 0, 0, 0.6)`，backdrop-filter: blur(4px)
- 弹窗：宽 640px，高 520px，居中
- 弹窗背景：`--bg-elevated`
- 边框：1px solid `--border-default`
- 圆角：`--radius-lg`
- 阴影：`--shadow-lg`
- 入场动画：scale(0.95) → scale(1) + opacity(0→1)，200ms

**弹窗头部：**
- 高度：52px
- 标题："设置"，18px，600 weight
- 关闭按钮 [✕]：右上角，24x24px，hover `--color-error`

**左侧导航（宽 140px）：**
- 背景：`--bg-surface`
- 分隔线：右 border 1px solid `--border-subtle`
- 导航项：高 40px，padding 0 16px，14px
  - 选中态：左侧 3px border `--accent-primary`，背景 `--bg-active`，文字 `--text-primary`
  - 普通态：文字 `--text-secondary`，hover 背景 `--bg-hover`

**右侧内容区：**
- padding：24px
- 可滚动

**表单控件规范：**

标签（Label）：
- 字体：13px，500 weight，`--text-secondary`
- 与输入框间距：6px

文本输入（Input）：
- 高度：40px
- 背景：`--bg-base`
- 边框：1px solid `--border-default`
- 圆角：`--radius-sm`
- padding：0 12px
- 字体：14px，`--font-mono`，`--text-primary`
- placeholder：`--text-muted`
- 聚焦态：边框 `--accent-primary`，`--shadow-glow`
- 字段间距：16px

下拉选择（Select）：
- 样式同文本输入
- 右侧下拉箭头

按钮：
- 主按钮（保存）：背景 `--accent-primary`，文字 `--text-inverse`，高 36px，圆角 `--radius-sm`，padding 0 20px，font-weight 600
- 次按钮（测试连接）：背景 transparent，边框 1px solid `--border-default`，文字 `--text-secondary`，hover 边框 `--accent-secondary`
- 按钮间距：8px

**设置项内容：**

1. **API 配置**
   - Provider（下拉：deepseek / openai / claude）
   - API Key（密码输入框，带"显示/隐藏"切换按钮）
   - Model（下拉：deepseek-chat / deepseek-reasoner / 自定义输入）
   - Base URL（可选，留空使用默认）
   - [测试连接] [保存]

2. **模型设置**
   - Temperature（滑块 0-2，默认 1.0）
   - Max Tokens（数字输入，默认 4096）
   - Stream（开关，默认开）

3. **记忆设置**
   - 最大会话消息数（数字输入，默认 100）
   - 压缩阈值 tokens（数字输入，默认 64000）
   - 自动压缩（开关，默认开）
   - 记忆检索 Top-K（数字输入，默认 3）

4. **关于**
   - 版本号：v0.3.0
   - 项目链接：GitHub
   - 许可证：MIT
   - AI 开发声明

---

### 3.13 动画与过渡

```css
/* ui/css/animations.css */

/* ===== 全局过渡 ===== */
button, a, input, textarea, select {
  transition: all var(--transition-normal);
}

/* ===== 消息入场 ===== */
@keyframes msgSlideIn {
  from {
    opacity: 0;
    transform: translateY(12px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.msg {
  animation: msgSlideIn 0.3s ease-out forwards;
}

/* 用户消息从右侧滑入 */
.msg-user {
  animation: msgSlideRight 0.3s ease-out forwards;
}

@keyframes msgSlideRight {
  from {
    opacity: 0;
    transform: translateX(12px);
  }
  to {
    opacity: 1;
    transform: translateX(0);
  }
}

/* ===== 打字指示器 ===== */
@keyframes typingDot {
  0%, 60%, 100% { opacity: 0.3; transform: translateY(0); }
  30% { opacity: 1; transform: translateY(-4px); }
}

.typing-indicator span {
  display: inline-block;
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--accent-primary);
  animation: typingDot 1.4s infinite;
}
.typing-indicator span:nth-child(2) { animation-delay: 0.2s; }
.typing-indicator span:nth-child(3) { animation-delay: 0.4s; }

/* ===== 工具调用卡片展开 ===== */
.tool-card-content {
  max-height: 0;
  overflow: hidden;
  transition: max-height 0.3s ease-out;
}
.tool-card.expanded .tool-card-content {
  max-height: 400px;
}

/* ===== 侧边栏会话项 ===== */
.session-item {
  transition: background var(--transition-fast),
              border-color var(--transition-fast),
              transform var(--transition-fast);
}
.session-item:active {
  transform: scale(0.98);
}

/* ===== 发送按钮脉冲 ===== */
@keyframes pulse {
  0% { box-shadow: 0 0 0 0 rgba(226, 165, 59, 0.4); }
  70% { box-shadow: 0 0 0 8px rgba(226, 165, 59, 0); }
  100% { box-shadow: 0 0 0 0 rgba(226, 165, 59, 0); }
}

.send-btn:not(:disabled):hover {
  animation: pulse 1.5s infinite;
}

/* ===== 空状态入场 ===== */
@keyframes fadeUp {
  from { opacity: 0; transform: translateY(20px); }
  to { opacity: 1; transform: translateY(0); }
}

.fade-up {
  opacity: 0;
  animation: fadeUp 0.5s ease-out forwards;
}
.fade-up-d1 { animation-delay: 0.1s; }
.fade-up-d2 { animation-delay: 0.2s; }
.fade-up-d3 { animation-delay: 0.3s; }
.fade-up-d4 { animation-delay: 0.5s; }

/* ===== 连接状态闪烁 ===== */
@keyframes blink {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.3; }
}
.status-connecting {
  animation: blink 1.2s infinite;
}
```

---

### 3.14 响应式适配

V0.3.0 暂不做移动端适配，但需要处理窗口缩放：

| 窗口宽度 | 侧边栏 | 聊天区 | 变化 |
|:---|:---|:---|:---|
| ≥ 1200px | 260px，正常显示 | 自适应 | 标准布局 |
| 960px - 1199px | 220px | 自适应 | 侧边栏略窄 |
| < 960px | 可折叠（汉堡菜单） | 全宽 | 侧边栏隐藏为抽屉 |

**折叠侧边栏：**
- 窗口宽度 < 960px 时，侧边栏默认隐藏
- 标题栏左侧出现汉堡菜单按钮（☰）
- 点击后侧边栏以 overlay 形式滑出（宽 280px，带遮罩层）
- 点击遮罩层或选择会话后自动收起

---

## 四、终端美化方案

V0.3.0 的终端美化采用渐进式方案：在现有 REPL 基础上增加样式化输出，不引入完整 TUI 框架（ratatui 留给 V0.4.0）。

### 4.1 方案选型

| 方案 | 复杂度 | 效果 | V0.3.0 选择 |
|:---|:---|:---|:---|
| A. 纯 ANSI 转义码 | 低 | 基础颜色 | ❌ 维护困难 |
| B. console + owo-colors crate | 中 | 颜色 + 样式 | ✅ 推荐 |
| C. ratatui 完整 TUI | 高 | 全功能 | 留给 V0.4.0 |

**新增依赖：**

```toml
[dependencies]
owo-colors = "4"       # 终端颜色
console = "0.15"       # 终端工具（宽度检测等）
indicatif = "0.17"     # 进度条/Spinner
```

### 4.2 配色方案

```
用户输入文字：  亮白色 (ANSI Bright White)
AI 回复文字：   默认白色
AI 回复强调：   琥珀色 (RGB 226, 165, 59)
工具调用标题：  青绿色 (RGB 78, 205, 196)
工具参数：     灰色 (ANSI Dark Gray)
工具结果：     默认白色
成功状态：     绿色 (ANSI Green)
错误状态：     红色 (ANSI Red)
警告状态：     黄色 (ANSI Yellow)
系统消息：     灰色 (ANSI Dark Gray)
分隔线：       灰色 (ANSI Dark Gray)
代码块背景：   无（终端不支持背景色块时用边框标记）
```

### 4.3 消息输出样式

**用户消息：**

```
  ╭─ You ─────────────────────────────────────╮
  │ 帮我列出桌面有哪些文件                      │
  ╰────────────────────────────────────────────╯
```

- 前缀 "╭─ You ─" 使用亮白色 + 粗体
- 边框字符使用灰色
- 消息文字使用亮白色

**AI 消息：**

```
  ╭─ RS-Claw ──────────────────────────────────╮
  │                                             │
  │ 你的桌面有以下文件：                         │
  │                                             │
  │   document.pdf   photo.jpg   project/       │
  │   notes.txt      music.mp3                  │
  │                                             │
  │ 共 12 个文件和 3 个文件夹。                  │
  │                                             │
  ╰────────────────────────────────────────────╯
```

- 前缀 "╭─ RS-Claw ─" 使用琥珀色 + 粗体
- 边框字符使用灰色
- 消息文字使用默认白色
- 强调文字（如数字、文件名）使用琥珀色

**工具调用：**

```
  ┌─ 🔧 fs_list ─────────── ✅ 0.3s ──────────┐
  │  → path: /home/user/Desktop                 │
  │  ← 12 items returned                       │
  └─────────────────────────────────────────────┘
```

- 工具名使用青绿色
- 状态使用绿色（成功）/ 红色（失败）/ 黄色（执行中）
- 参数箭头 → 使用灰色
- 结果箭头 ← 使用灰色

**系统消息：**

```
  ── 会话已恢复，共 12 条历史消息 ──
```

- 灰色，居中（在终端宽度内居中）

### 4.4 Spinner（工具执行中）

使用 `indicatif` crate 的 spinner：

```
  ⠋ 正在调用 fs_list...
```

- Spinner 字符：⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏（braille 系列）
- 颜色：青绿色
- 文字：灰色
- 工具完成后 spinner 消失，替换为结果

### 4.5 REPL 提示符美化

**输入提示符：**

```
  ❯ _
```

- "❯" 使用琥珀色
- 光标闪烁

**多行输入提示：**

```
  ❯ 第一行
  │ 第二行
  │ _
```

- 续行前缀 "│" 使用灰色

### 4.6 命令输出美化

**/help 输出：**

```
  ┌─ RS-Claw v0.3.0 ── 命令帮助 ──────────────┐
  │                                             │
  │  /help      显示此帮助                      │
  │  /quit      退出（自动保存会话）             │
  │  /clear     开始新会话                      │
  │  /config    查看当前配置                     │
  │  /tools     列出可用工具                     │
  │  /sessions  查看历史会话                     │
  │                                             │
  │  直接输入消息开始对话                        │
  │                                             │
  └─────────────────────────────────────────────┘
```

**/tools 输出：**

```
  ┌─ 可用工具 ─────────────────────────────────┐
  │                                             │
  │  📁 fs_read      读取文件内容               │
  │  📁 fs_write     写入文件                   │
  │  📁 fs_list      列出目录                   │
  │  📁 fs_exists    检查文件是否存在            │
  │  💻 shell        执行 Shell 命令            │
  │  🌐 http_get     HTTP GET 请求              │
  │  🌐 http_post    HTTP POST 请求             │
  │                                             │
  │  共 7 个工具                                │
  └─────────────────────────────────────────────┘
```

---

## 五、Gateway 协议扩展

V0.3.0 需要扩展 WebSocket JSON-RPC 协议以支持 GUI 所需的新 method：

### 5.1 新增 Method

| Method | 方向 | 说明 |
|:---|:---|:---|
| `session.list` | req → res | 获取会话列表 |
| `session.create` | req → res | 创建新会话 |
| `session.delete` | req → res | 删除会话 |
| `session.get` | req → res | 获取单个会话详情（含消息历史） |
| `memory.list` | req → res | 查询记忆列表 |
| `memory.search` | req → res | 向量搜索记忆 |
| `skill.list` | req → res | 获取技能列表 |
| `config.get` | req → res | 获取配置 |
| `config.update` | req → res | 更新配置 |

### 5.2 流式事件扩展

现有事件类型：

```json
{"type":"event","event":"agent","payload":{"stream":"assistant","data":{"text":"...","finish_reason":"stop"}}}
```

新增事件类型：

```json
{"type":"event","event":"tool_call","payload":{"id":"tc_1","name":"fs_list","arguments":"{...}"}}
{"type":"event","event":"tool_result","payload":{"id":"tc_1","result":"...","duration_ms":300}}
{"type":"event","event":"reasoning","payload":{"text":"让我分析一下..."}}
{"type":"event","event":"session_restored","payload":{"session_id":"...","message_count":12}}
```

---

## 六、L4/L5 预留接口

### 6.1 L4 桌面操控 trait

```rust
// src/tools/desktop.rs (V0.3.0 仅定义 trait，不实现)

use async_trait::async_trait;
use crate::tools::ToolResult;

/// 桌面操控能力接口
/// V0.4.0+ 将基于 AT-SPI2 (Linux) / Accessibility API (Windows) 实现
#[async_trait]
pub trait DesktopControl: Send + Sync {
    /// 列出桌面上所有可见窗口
    async fn list_windows(&self) -> ToolResult<Vec<WindowInfo>>;
    
    /// 获取指定窗口的 UI 树
    async fn get_ui_tree(&self, window_id: &str) -> ToolResult<UITreeNode>;
    
    /// 点击指定元素
    async fn click_element(&self, element_id: &str) -> ToolResult<()>;
    
    /// 在指定元素中输入文字
    async fn type_text(&self, element_id: &str, text: &str) -> ToolResult<()>;
    
    /// 截取屏幕截图（供视觉模型分析）
    async fn screenshot(&self, window_id: Option<&str>) -> ToolResult<Vec<u8>>;
    
    /// 按键操作
    async fn press_key(&self, key: &str) -> ToolResult<()>;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WindowInfo {
    pub id: String,
    pub title: String,
    pub app_name: String,
    pub position: (i32, i32),
    pub size: (u32, u32),
    pub is_focused: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UITreeNode {
    pub role: String,          // "button", "text", "menu", etc.
    pub name: Option<String>,
    pub description: Option<String>,
    pub children: Vec<UITreeNode>,
    pub bounds: Option<(i32, i32, u32, u32)>,
    pub is_actionable: bool,
}
```

### 6.2 L5 浏览器操控 trait

```rust
// src/tools/browser.rs (V0.3.0 仅定义 trait，不实现)

use async_trait::async_trait;
use crate::tools::ToolResult;

/// 浏览器操控能力接口
/// V0.4.0+ 将基于 Chrome DevTools Protocol (CDP) 实现
#[async_trait]
pub trait BrowserControl: Send + Sync {
    /// 打开 URL
    async fn navigate(&self, url: &str) -> ToolResult<()>;
    
    /// 获取当前页面 HTML
    async fn get_page_content(&self) -> ToolResult<String>;
    
    /// 获取页面文本内容（去 HTML 标签）
    async fn get_page_text(&self) -> ToolResult<String>;
    
    /// 执行 JavaScript
    async fn execute_js(&self, script: &str) -> ToolResult<String>;
    
    /// 点击页面元素（CSS 选择器）
    async fn click(&self, selector: &str) -> ToolResult<()>;
    
    /// 在输入框中填写文字
    async fn fill(&self, selector: &str, value: &str) -> ToolResult<()>;
    
    /// 截取页面截图
    async fn screenshot(&self) -> ToolResult<Vec<u8>>;
    
    /// 获取当前 URL
    async fn get_url(&self) -> ToolResult<String>;
}
```

### 6.3 Feature Gate 配置

```toml
# Cargo.toml
[features]
default = []
desktop = []    # L4 桌面操控
browser = []    # L5 浏览器操控
```

GUI 中预留 L4/L5 入口但标记为"即将推出"：

```
┌──────────────────────┐
│                      │
│  [＋ 新对话]          │
│                      │
│  会话列表...          │
│                      │
│  ────────────────    │
│  🧠 3 条记忆         │
│  ⚡ 5 个技能         │
│                      │
│  ── 即将推出 ─────   │  ← 新增分组
│  🖥️ 桌面操控  🔒     │  ← 灰色，不可点击
│  🌐 浏览器    🔒     │  ← 灰色，不可点击
│                      │
│  ⚙ 设置              │
│  v0.3.0              │
└──────────────────────┘
```

- "即将推出" 分组标题：11px，`--text-muted`
- 功能项：灰色（`--text-muted`），带锁图标 🔒
- 不可点击，hover 显示 tooltip："V0.4.0 即将推出"

---

## 七、开发里程碑

### Phase 1：项目重构（预计 1-2 天）

| 任务 | 说明 |
|:---|:---|
| 提取 `lib.rs` | 将核心逻辑从 `main.rs` 提取为库 |
| Workspace 配置 | 创建 workspace Cargo.toml |
| Tauri 骨架 | `src-tauri/` 初始化，空窗口能运行 |
| 验证 | CLI 和 GUI 都能正常编译运行 |

### Phase 2：GUI 基础框架（预计 2-3 天）

| 任务 | 说明 |
|:---|:---|
| HTML/CSS 骨架 | 布局框架（标题栏 + 侧边栏 + 主区） |
| 设计 token | `variables.css` 完整实现 |
| 自定义标题栏 | 可拖动 + 窗口控制 |
| Tauri IPC | 基础 commands 注册 |

### Phase 3：聊天核心（预计 3-4 天）

| 任务 | 说明 |
|:---|:---|
| 消息渲染 | 用户消息 + AI 消息 + Markdown |
| 流式输出 | Tauri 事件 → 逐字渲染 |
| 工具调用卡片 | 展开/收起 + 状态展示 |
| 输入区 | 自动扩展 + 快捷键 |
| 打字指示器 | AI 生成时的动画 |

### Phase 4：侧边栏与会话管理（预计 2 天）

| 任务 | 说明 |
|:---|:---|
| 会话列表 | 时间分组 + 选中态 |
| 新建/切换/删除会话 | 完整 CRUD |
| 空状态页 | 首次使用引导 |
| 记忆/技能统计 | 底部数字展示 |

### Phase 5：设置面板（预计 1-2 天）

| 任务 | 说明 |
|:---|:---|
| 模态弹窗 | 遮罩 + 动画 |
| API 配置表单 | Provider/Key/Model |
| 测试连接功能 | 验证 API 可用性 |
| 记忆/模型设置 | 参数调整 |

### Phase 6：终端美化（预计 1-2 天）

| 任务 | 说明 |
|:---|:---|
| 消息框样式 | 用户/AI 消息边框 |
| 工具调用样式 | 彩色输出 |
| Spinner | 工具执行中动画 |
| 命令输出美化 | /help, /tools 等 |

### Phase 7：收尾与预留（预计 1 天）

| 任务 | 说明 |
|:---|:---|
| L4/L5 trait 定义 | 接口文件创建 |
| "即将推出" UI | 侧边栏占位 |
| 响应式适配 | 窗口缩放处理 |
| 测试与修复 | 端到端验证 |

**总计预计：11-16 天**

---

## 八、关键技术决策记录

| 决策 | 选择 | 理由 |
|:---|:---|:---|
| GUI 框架 | Tauri v2 | 系统 WebView，无额外运行时，HTML/CSS 设计自由度高 |
| 前端技术 | Vanilla HTML/CSS/JS | 项目规模不需要框架，减少复杂度 |
| 通信方式 | Tauri IPC + Events | 直接调用 Rust，无需经过 WebSocket |
| 终端美化 | owo-colors + indicatif | 渐进式改进，不引入 TUI 框架 |
| Markdown 渲染 | marked.js | 轻量、成熟 |
| 代码高亮 | highlight.js | 支持语言多、主题丰富 |
| 图标方案 | 内联 SVG | 无外部依赖，可控 |
| 字体 | JetBrains Mono + IBM Plex Sans | 开发者工具气质，区别于通用 UI |

---

*RS-Claw V0.3.0 技术方案 · 2026 年 4 月*
