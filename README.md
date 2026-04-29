# RS-Claw

> Rust 重写的跨平台 AI 电脑助手 — 零运行时依赖，4MB 单文件，双击即用。

---

## 🤖 AI 开发声明

**本项目 100% 由 AI 辅助开发完成。**

从技术选型、模块架构设计、代码编写、编译调试、到文档撰写——全程由 **DeepSeek V4 Pro** 与人类（[yunhaohao59-cpu](https://github.com/yunhaohao59-cpu)）对话协作完成。没有 AI 就没有这个项目。

这也是本项目最想传达的一点：**未来已来，每个人都可以和 AI 一起创造。**

特别鸣谢：
- [OpenClaw](https://github.com/openclaw/openclaw) — 本项目的基础与灵感来源，MIT 协议
- DeepSeek V4 Pro — 本项目 100% 由你协助开发完成 🫡

---

## 这是什么

RS-Claw 是基于 [OpenClaw](https://github.com/openclaw/openclaw) 的 Rust 重写版本。

OpenClaw 是一个优秀的 Node.js AI 助手项目，功能强大但安装依赖 Node.js、npm、复杂的配置流程，对普通用户门槛太高。RS-Claw 保留了 OpenClaw 的核心架构思想（Gateway Hub + Agent 运行时 + 工具系统），用 Rust 从头实现了一遍。

RS-Claw 做了三件事：

1. **Rust 重写** — 编译为单一 4MB 二进制文件
2. **零依赖** — 不需要 Node.js、Python 或任何运行时
3. **双击即用** — 配置一次 API Key，像聊天一样使用

你只需要用自然语言告诉它要干什么，它会自动调用工具去操作电脑，然后告诉你结果。

---

## 架构

```
┌──────────┐     ┌───────────┐     ┌──────────┐
│   CLI    │────▶│  Gateway   │────▶│  Agent   │
│  (REPL)  │     │ (WS JSON) │     │ Runtime  │
└──────────┘     └───────────┘     └────┬─────┘
                                        │
                              ┌─────────┼─────────┐
                              │         │         │
                          ┌───▼──┐ ┌───▼──┐ ┌───▼──┐
                          │Model │ │Tools │ │Memory│
                          │Layer │ │ L1-L3│ │System│
                          └──────┘ └──────┘ └──────┘
```

- **CLI** — 交互式 REPL + 单次命令模式
- **Gateway** — WebSocket JSON-RPC 服务器，支持多客户端接入
- **Agent Runtime** — 核心循环：接收消息 → LLM 推理 → 工具调用 → 结果注入 → 继续推理（最多 10 轮）
- **Model Layer** — 统一抽象，支持 DeepSeek / OpenAI / Claude
- **Tools** — 7 个内置工具，覆盖文件、进程、网络操作
- **Memory** — 会话记忆 + 向量检索（规划中）

---

## 核心能力

| 层级 | 工具 | 说明 |
|:---|:---|:---|
| L1 文件系统 | `fs_read` `fs_write` `fs_list` `fs_exists` | 读、写、列、检查文件 |
| L2 进程管理 | `shell` | 执行 Shell 命令（Linux sh / Win cmd） |
| L3 网络请求 | `http_get` `http_post` | HTTP 请求，自动截断过长的响应 |
| L4 桌面控制 | 🚧 规划中 | AT-SPI2 + 视觉模型 |
| L5 浏览器 | 🚧 规划中 | CDP 协议操控 Chromium |

---

## 快速开始

### 1. 下载

从 [Releases](https://github.com/yunhaohao59-cpu/rs-claw-one/releases) 下载对应平台的版本：

| 平台 | 文件 |
|:---|:---|
| Windows 11 x64 | `rs-claw.exe` |
| Linux x86_64 | `rs-claw` |

### 2. 获取 API Key

访问 [platform.deepseek.com](https://platform.deepseek.com) 注册并创建 API Key。10 块钱用很久。

### 3. 配置

```bash
# Windows
rs-claw.exe setup

# Linux
./rs-claw setup
```

按提示输入 Provider、API Key 和 Model，配置会保存在 `~/.rs-claw/config.toml`。

### 4. 开始使用

```bash
# 交互式对话（推荐）
rs-claw

# 单次命令
rs-claw chat -m "帮我列出桌面有哪些文件"
```

---

## 配置文件

`~/.rs-claw/config.toml`：

```toml
[gateway]
port = 18789
host = "127.0.0.1"

[model]
provider = "deepseek"
api_key = "sk-your-key-here"
model = "deepseek-chat"

[memory]
max_session_messages = 100
compaction_threshold = 64000
auto_compaction = true
```

支持环境变量 `${VAR_NAME}` 引用（用于 CI/CD 等场景）。

---

## Gateway 模式

启动 WebSocket 服务器，允许其他程序接入：

```bash
./rs-claw serve --port 18789
```

客户端通过 JSON-RPC 帧通信：

```json
// 发送消息
{"type":"req","id":"1","method":"chat.send","params":{"message":"你好"}}

// 收到回复
{"type":"res","id":"1","ok":true,"payload":{"session_key":"..."}}
{"type":"event","event":"agent","payload":{"stream":"assistant","data":{"text":"你好！","finish_reason":"stop"}}}
```

---

## 从源码编译

```bash
# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 克隆仓库
git clone git@github.com:yunhaohao59-cpu/rs-claw-one.git
cd rs-claw-one

# 编译 Linux 版本
cargo build --release

# 交叉编译 Windows 版本（需要 mingw-w64）
rustup target add x86_64-pc-windows-gnu
sudo apt install mingw-w64
cargo build --release --target x86_64-pc-windows-gnu
```

---

## 技术栈

- **语言**：Rust (edition 2021)
- **异步运行时**：Tokio
- **WebSocket**：tokio-tungstenite
- **HTTP 客户端**：reqwest
- **CLI**：clap
- **序列化**：serde + serde_json + toml
- **数据库**：rusqlite (SQLite)
- **日志**：tracing + tracing-subscriber

---

## 路线图

- [x] CLI REPL + 单次命令
- [x] Gateway WebSocket JSON-RPC
- [x] DeepSeek / OpenAI / Claude 模型层
- [x] L1 文件系统工具
- [x] L2 Shell 执行工具
- [x] L3 HTTP 请求工具
- [x] Agent 工具调用循环
- [x] 会话记忆
- [x] System Prompt + 上下文文件注入
- [x] Windows 交叉编译
- [ ] 向量记忆检索
- [ ] 上下文压缩（Compaction）
- [ ] 技能自动提炼引擎
- [ ] L4 桌面 GUI 操控
- [ ] L5 浏览器操控
- [ ] TUI 终端界面
- [ ] macOS 支持

---

## 协议

MIT License © 2025 [yunhaohao59-cpu](https://github.com/yunhaohao59-cpu)

本项目基于 [OpenClaw](https://github.com/openclaw/openclaw)（MIT License © 2025 OpenClaw contributors）重写，保留原始版权声明，详见 [LICENSE](./LICENSE)。
