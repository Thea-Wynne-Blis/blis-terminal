# Blis Terminal

一款 Windows 上的个人命令终端：常驻托盘，`Alt+Space` 随时唤出，在 `blis>` 提示符下管理目录、文件、笔记，搜索网页，保存常用应用。

内置命令：`help` `search` `open` `pwd` `cd` `ls` `code` `mkdir` `cp` `mv` `rm` `background` `welcome` `alias` `calc` `date` `timer` `pick` `todo` `game` `note` `history` `clear` `version` `shortcut` `echo` `exit`。
每条命令都支持 `help <命令>` 或 `<命令> --help`。

`game list` 查看终端里的小游戏。`game snake` 用方向键或 WASD 移动，空格暂停。`game runner` 用空格或 ↑ 跳跃，跑过的距离计入成绩。`game dodge` 用 ←／→ 或 A／D 左右移动，按躲过的掉落物计分。跑酷和躲避都用 P 暂停或继续。三款游戏隐藏窗口后会暂停，结束后按 R 重开，Esc 返回命令行，最高纪录保存在本机。

输入 `code .` 可在 VS Code 中打开 Blis 的当前目录。`code README.md` 打开当前目录下的文件，路径含空格时用引号包住。输入部分路径后可按 Tab 补全。

- `timer 25m "写代码"` 设置后台倒计时。单位为 `d`、`h`、`m`、`s`，隐藏到托盘后仍会在到点时弹出提醒。
- `pick 火锅 烧烤 拉面` 从给定选项里随机选一个。选项名含空格时加引号。
- `todo add "补发版说明"` 添加待办，`todo list` 查看未完成项，`todo done 3` 标记编号 3 完成。待办保存到应用配置目录中的 `todos.json`。
- `welcome set "欢迎回来！"` 修改并保存终端顶部的欢迎语。`welcome` 查看当前欢迎语，`welcome reset` 恢复默认。

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
  terminal/TerminalView.vue       输入、历史、输出和弹窗
  terminal/terminal-commands.ts   命令表、别名展开和 Tab 补全
  features/                     各功能的命令、界面、规则和测试
  platform/desktop-platform.ts    桌面 IPC 桥
          │
          │  Tauri invoke
          ▼
Rust + Tauri 2
  main.rs                插件与命令注册、状态装配
  app/lifecycle.rs       托盘、全局快捷键和窗口显隐
  features/              文件、外观、待办、搜索、笔记和别名
  platform/              Windows 打开目标、进程检测和配置写入
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
│  │  ├─ terminal/             输入、历史、快捷键、命令表和共享类型
│  │  ├─ features/
│  │  │  ├─ games/             Snake、Runner、Dodge 的命令、规则、画面和测试
│  │  │  ├─ files/             目录和文件命令、路径补全
│  │  │  ├─ appearance/        背景、透明度和欢迎语
│  │  │  ├─ notes/             笔记命令与编辑器
│  │  │  ├─ search/            搜索命令
│  │  │  ├─ tasks/             倒计时、提醒和待办
│  │  │  └─ tools/             别名、计算、时间和随机选择
│  │  ├─ components/           多个功能共用的界面组件
│  │  └─ platform/             桌面 IPC 调用
│  └─ src-tauri/                Rust 后端
│     ├─ src/main.rs            插件、命令注册、状态装配
│     ├─ src/app/               托盘、全局快捷键和窗口生命周期
│     ├─ src/features/          各命令的系统操作和设置
│     ├─ src/platform/          Windows 调用和共用存储工具
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
