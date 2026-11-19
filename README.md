# Blis Terminal

一款 Windows 上的个人命令终端：常驻托盘，`Alt+Space` 随时唤出，在 `blis>` 提示符下管理目录、文件、笔记，搜索网页，保存常用应用。

内置命令：`help` `search` `open` `pwd` `cd` `ls` `mkdir` `cp` `mv` `rm` `background` `alias` `calc` `date` `note` `history` `clear` `version` `shortcut` `echo` `exit`。
每条命令都支持 `help <命令>` 或 `<命令> --help`。

## 技术栈

| 层       | 选型                               |
| -------- | ---------------------------------- |
| 界面     | Vue 3 + TypeScript + Vite          |
| 桌面外壳 | Tauri 2（Rust）+ 系统 WebView2     |
| 工程     | pnpm workspace、ESLint、Prettier   |
| 测试     | Vitest（前端）、cargo test（后端） |

## 架构

一句话概括：**前端管"命令怎么写"，Rust 管"系统怎么干"。**

```
Vue 3 + WebView2
  TerminalView.vue       终端外壳：输入、历史、输出、弹窗
  terminal-commands.ts   命令表、别名展开、Tab 补全
  *-command.ts           各命令的参数解析 → CommandResult
  desktop-platform.ts    IPC 桥
          │
          │  Tauri invoke
          ▼
Rust + Tauri 2
  main.rs                插件与命令注册、状态装配
  lifecycle.rs           托盘、全局快捷键、窗口显隐
  directories.rs / notes.rs / aliases.rs / …   业务实现
  %APPDATA%/dev.blis.terminal/                 JSON 与笔记存储
```

### 一条命令的执行路径

以 `mkdir "项目 备份"` 为例：

1. 回车后 `TerminalView.vue` 的 `submitCommand` 调用 `executeTerminalInput`
2. `terminal-commands.ts` 查命令表，命中 `mkdir`，把原始参数交给它的 `run`
3. `run` 解析参数，返回 `CommandResult`：
   - `{ type: 'text' | 'error' }`、`{ type: 'clear' }` —— 前端自己就能处理
   - `{ type: 'directory', operation }` —— 需要系统能力
4. `completeDesktopCommand` 经 `desktop-platform.ts` 发出 `invoke('execute_directory', …)`
5. Rust 执行真实的文件操作，返回 `DirectoryOutcome`
6. 结果写回 transcript，渲染到屏幕

### 几个设计取舍

- **命令定义即文档**：`CommandDefinition` 把 `summary` / `usage` / `examples` / `run` 放在一起，`help` 和 Tab 补全都从同一张表生成，不会出现"文档忘了改"。
- **能力显式化**：前端不直接碰文件系统，只产生 `command-types.ts` 里的 operation 枚举；这份类型就是前后端唯一的契约。
- **状态集中在 Rust**：当前目录、别名、笔记、搜索站点、背景设置都在后端，重启后自然恢复。
- **进程常驻**：单实例 + 托盘 + 全局快捷键；关闭按钮只是隐藏窗口，再次唤回复用同一个窗口，输入不丢。
- **收窄权限**：发布页面启用 CSP；`capabilities/terminal.json` 用白名单只放开列出的业务命令，没有引入任意 Shell 或文件系统插件。

## 项目结构

```text
blis-terminal/
├─ apps/desktop/
│  ├─ src/                      Vue 前端
│  │  ├─ TerminalView.vue       终端外壳：输入、历史、输出、弹窗
│  │  ├─ terminal-commands.ts   命令表、解析入口、补全
│  │  ├─ *-command.ts           各命令的参数解析（directory / search / note …）
│  │  ├─ desktop-platform.ts    IPC 桥
│  │  ├─ command-types.ts       前后端共享的 operation 契约
│  │  ├─ terminal-keyboard.ts   快捷键与输入历史
│  │  └─ *.test.ts              Vitest 单测
│  └─ src-tauri/                Rust 后端
│     ├─ src/main.rs            插件、命令注册、状态装配
│     ├─ src/lifecycle.rs       托盘、全局快捷键、窗口生命周期
│     ├─ src/directories.rs     目录与路径操作
│     ├─ src/file_operations.rs mkdir / cp / mv / rm（回收站）
│     ├─ src/notes.rs           笔记读写与导出
│     ├─ src/aliases.rs         命令别名
│     ├─ src/search_settings.rs 搜索站点与浏览器
│     ├─ src/appearance.rs      背景主题与不透明度
│     └─ tauri.conf.json        窗口、CSP、NSIS 打包配置
├─ docs/phase-0.md              架构决策与实测记录
├─ scripts/                     内存测量脚本
└─ .github/workflows/windows.yml
```

## 本地开发

需要 Node.js（见 `.node-version`）、pnpm 11、Rust（见 `rust-toolchain.toml`），以及带「使用 C++ 的桌面开发」工作负载的 Build Tools 和 WebView2 Runtime，详见 [Tauri 环境要求](https://v2.tauri.app/start/prerequisites/)。

```powershell
npm install --global pnpm@11.25.0
pnpm install
pnpm tauri dev
```

只想调界面可以用 `pnpm dev` 在浏览器里预览；托盘、快捷键、文件系统等能力需要桌面版。

快捷键：`Alt+Space` 唤出/隐藏（冲突时退到 `Alt+Shift+Space`），`Tab` 补全，`Ctrl+L` 清屏，`Ctrl+O` 折叠输出，`Esc` 关闭弹窗。

## 检查与构建

```powershell
pnpm lint
pnpm format:check
pnpm test
pnpm build
cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml --check
cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --locked --all-targets -- -D warnings
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked
pnpm bundle
```

`pnpm bundle` 会打包 Windows 安装包，输出在 `apps/desktop/src-tauri/target/release/bundle/nsis/`。

## 发版

推一个 `v*` tag 即可，CI 会构建并在构建成功后自动创建 Release、上传安装包：

```powershell
git tag v0.2.1
git push origin v0.2.1
```

普通分支推送只跑检查与构建，不会发版。更多背景见 [Phase 0 验证记录](docs/phase-0.md)。

## 许可证

[Apache-2.0](LICENSE)
