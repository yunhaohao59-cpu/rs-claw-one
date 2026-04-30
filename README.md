# RS-Claw

> Rust 重写的跨平台 AI 电脑助手 — 零运行时依赖，5-6 MB 单文件，双击即用。
> 会学习，会记忆，不健忘。

---

## 🤖 AI 开发声明

**本项目 100% 由 AI 辅助开发完成。**

从技术选型、模块架构设计、代码编写、编译调试、到文档撰写——全程由 **DeepSeek V4 Pro** 与人类（[yunhaohao59-cpu](https://github.com/yunhaohao59-cpu)）对话协作完成。

特别鸣谢：
- [OpenClaw](https://github.com/openclaw/openclaw) — 架构基础与灵感来源（MIT License）
- DeepSeek V4 Pro — 核心开发伙伴

---

## 这是什么

RS-Claw 是基于 [OpenClaw](https://github.com/openclaw/openclaw) 的 Rust 重写版本。

OpenClaw 是一个优秀的 Node.js AI 助手项目，但安装依赖 Node.js、npm，对普通用户门槛太高。RS-Claw 保留了其核心架构（Gateway Hub + Agent + 工具系统），用 Rust 从头重写。

**你只需要打字告诉它要干什么，它就去帮你操作电脑。**

---

## 架构

```
用户消息 → System Prompt + 工具定义 + 记忆注入 + 对话历史
         → DeepSeek 推理
         → 需要工具? → fs_read/shell/http_get... → 结果注入 → 继续推理 (max 10轮)
         → 不需要  → 返回最终回复 → 自动提炼技能 → 存入 SQLite + VectorStore
```

---

## 核心能力

### 7 个内置工具

| 工具 | 说明 |
|:---|:---|
| `fs_read` | 读取文件内容 |
| `fs_write` | 写入文件 |
| `fs_list` | 列出目录 |
| `fs_exists` | 检查文件是否存在 |
| `shell` | 执行 Shell / cmd 命令 |
| `http_get` | HTTP GET 请求 |
| `http_post` | HTTP POST 请求 |

### 记忆与学习系统

| 能力 | 说明 |
|:---|:---|
| SQLite 持久化 | 对话自动保存，启动自动恢复 |
| 技能自动提炼 | 完成任务后 LLM 提取可复用技能 |
| 向量记忆检索 | trigram 256维 embedding，自动注入相关记忆 |
| 上下文压缩 | Token 超阈值自动 LLM 摘要，不怕长对话 |

---

## 快速开始

### 1. 下载

从 [Releases](https://github.com/yunhaohao59-cpu/rs-claw-one/releases) 下载对应平台：

| 平台 | 架构 | 文件 |
|:---|:---|:---|
| Linux | x86_64 | `rs-claw` |
| Linux | ARM64 | `rs-claw-arm64` |
| Windows | x64 | `rs-claw.exe` |

### 2. 获取 API Key

访问 [platform.deepseek.com](https://platform.deepseek.com) 注册并创建 API Key。10 块钱用很久。

### 3. 配置

```bash
# Windows
rs-claw.exe setup

# Linux
./rs-claw setup
```

按提示输入 Provider（直接回车）、API Key、Model（直接回车或指定），配置保存到 `~/.rs-claw/config.toml`。

### 4. 开始使用

```bash
# 交互式对话（推荐）
rs-claw

# 单次命令
rs-claw chat -m "帮我列出桌面有哪些文件"

# 启动 Gateway 服务器
rs-claw serve --port 18789
```

---

## REPL 命令

| 命令 | 作用 |
|:---|:---|
| `/help` | 帮助 |
| `/quit` | 退出（自动保存会话） |
| `/clear` | 新会话 |
| `/config` | 查看配置 |
| `/tools` | 列出可用工具 |
| `/sessions` | 查看历史会话 |

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
top_k_memories = 3

[skill]
auto_refine = true
similarity_threshold = 0.75
```

支持 `${ENV_VAR}` 环境变量引用。

---

## Gateway 模式

```bash
./rs-claw serve --port 18789
```

WebSocket JSON-RPC：

```json
{"type":"req","id":"1","method":"chat.send","params":{"message":"你好"}}
{"type":"event","event":"agent","payload":{"stream":"assistant","data":{"text":"你好！","finish_reason":"stop"}}}
```

---

## 从源码编译

```bash
git clone git@github.com:yunhaohao59-cpu/rs-claw-one.git
cd rs-claw-one

# Linux x86_64
cargo build --release

# Linux ARM64 (需 aarch64-linux-gnu-gcc)
rustup target add aarch64-unknown-linux-gnu
sudo apt install gcc-aarch64-linux-gnu g++-aarch64-linux-gnu
CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc cargo build --release --target aarch64-unknown-linux-gnu

# Windows x64 (需 mingw-w64)
rustup target add x86_64-pc-windows-gnu
sudo apt install mingw-w64
cargo build --release --target x86_64-pc-windows-gnu

# macOS (需 macOS 机器或 zig + macOS SDK)
cargo zigbuild --release --target x86_64-apple-darwin
cargo zigbuild --release --target aarch64-apple-darwin
```

---

## 技术栈

- **语言**：Rust 2021
- **异步**：Tokio
- **WebSocket**：tokio-tungstenite
- **HTTP**：reqwest (rustls)
- **CLI**：clap
- **数据库**：rusqlite (bundled SQLite)
- **向量检索**：trigram embedding (256维)
- **跨平台编译**：mingw-w64 + zig
- **日志**：tracing

---

## 路线图

- [x] CLI REPL + 单次命令
- [x] Gateway WebSocket JSON-RPC
- [x] 7 个内置工具
- [x] Agent 工具调用循环
- [x] SQLite 对话持久化
- [x] 向量记忆检索
- [x] Compaction 上下文压缩
- [x] 技能自动提炼引擎
- [x] Windows x64 交叉编译
- [x] Linux ARM64 交叉编译
- [ ] L4 桌面 GUI 操控
- [ ] L5 浏览器操控
- [ ] TUI 终端界面
- [ ] macOS 编译
- [ ] CI/CD 自动发布

---

## 协议

MIT License © 2025 [yunhaohao59-cpu](https://github.com/yunhaohao59-cpu)

本项目基于 [OpenClaw](https://github.com/openclaw/openclaw)（MIT License © 2025 OpenClaw contributors）重写，详见 [LICENSE](./LICENSE)。
