# Blis Terminal

面向 Windows 10/11 x64 的个人命令终端。启动后在 `blis>` 提示符输入 `help`，可查看当前的内置命令。使用上下方向键浏览本次会话的输入历史。

每条命令都可以用 `help <命令>`、`<命令> --help` 或 `<命令> -h` 查看详细用法和示例。命令的说明与执行函数一起注册，命令列表和详细帮助从同一份定义生成。

输入 `shortcut` 可查看快捷键。`Ctrl+L` 清屏，`Ctrl+O` 折叠或展开已执行命令的输出。可用鼠标拖选屏幕上的命令或 `history` 输出，再按 `Ctrl+C` 复制。上下方向键可取回历史命令继续编辑。

新增命令时实现 `CommandDefinition`，再加入 `apps/desktop/src/terminal-commands.ts` 的命令表。新增搜索引擎时，在 `apps/desktop/src/search-command.ts` 为引擎填写标识、名称、搜索入口和关键词参数，帮助会自动列出它。

## 搜索与打开网页

```text
search Rust 入门教程
search github "vue terminal"
search "Windows 托盘" --engine=bing
search -e google "Rust Tauri"
search https://github.com/liyupi/yuindex
```

省略引擎时使用百度。可选引擎为 `baidu`、`bing`、`google`、`github`。引号用于把带空格的内容作为一个参数，`--` 之后按关键词处理，例如 `search bing -- -example`。直接跳转接受 HTTP 和 HTTPS 网址。搜索结果和网址均交给系统默认浏览器打开。输入 `help search` 可查看用法。

## 保存常用打开目标

```text
open add vscode --app "D:/Apps/Microsoft VS Code/Code.exe"
open add 项目 --folder "G:/web-ai-code/blis-terminal"
open add 报告 --file "D:/Documents/报告.pdf"
open add github --url https://github.com
open vscode
open list
open remove 报告
help open
```

将示例路径替换为自己的绝对路径，含空格时加引号。别名可用中文，英文字母不区分大小写。同名目标可先移除再添加。`open remove` 只删除清单记录。清单保存于 `%APPDATA%/dev.blis.terminal/open-targets.json`，重启后仍可使用。

`open <别名>` 在等待打开时显示进度指示。Windows 应用检测到可见窗口后显示「已打开」。启动器提前退出或 8 秒内还未检测到窗口时，会显示已知的启动状态。文件、文件夹和网址会提示已交给系统打开。

## 背景设置

```text
background
background add 夜景 --image "D:/壁纸/夜景.jpg"
background add 海蓝 --color #172942
background opacity 65
background remove 夜景
background reset
help background
```

用 `background` 打开菜单，按 `↑`、`↓` 选择，按 Enter 应用，按 Esc 关闭。深色和亮色始终保留。自定义背景、当前选择和不透明度会保存在用户配置目录。不透明度 0 为完全透明，100 为完全不透明，文字的不透明度保持 100%。重置恢复深色和 100% 不透明度，自定义背景仍可选择。

## 本地运行

使用 `.node-version` 中的 Node.js、`package.json` 中的 pnpm 和 `rust-toolchain.toml` 中的 Rust。Windows 需要带有「使用 C++ 的桌面开发」工作负载的 Microsoft C++ Build Tools，以及 WebView2 Runtime。[Tauri 环境要求](https://v2.tauri.app/start/prerequisites/)

```powershell
npm install --global pnpm@11.25.0
pnpm install --frozen-lockfile
pnpm tauri dev
```

- 默认全局唤醒键是 `Alt+Space`。注册失败时尝试 `Alt+Shift+Space`，窗口会显示实际生效的快捷键。两个都不可用时可单击托盘图标。
- 首次打开时，窗口在鼠标所在显示器的可用区域内居中，默认约占 50% 宽、63% 高。可拖拽窗口边缘或角落调整大小，也可以最大化。隐藏后重新唤醒会保留当前尺寸。
- `Esc` 和原生窗口关闭按钮会隐藏窗口。再次运行同一程序会唤醒原实例。输入 `exit` 或选择托盘菜单中的「退出」会结束进程，输入 `help exit` 可查看说明。
- 直接启动 Release 可执行文件时，可加 `--background` 验证首次启动只初始化托盘和快捷键。第一次唤醒才创建窗口，隐藏后复用窗口并保留输入。
- `pnpm dev` 启动浏览器预览，用于检查布局。桌面行为通过 `pnpm tauri dev` 或 Release 可执行文件验证。

## 检查与构建

```powershell
pnpm lint
pnpm format:check
pnpm test
pnpm build
cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml --check
cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --locked --all-targets -- -D warnings
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked
pnpm tauri build -- --locked
```

NSIS 安装器输出在 `apps/desktop/src-tauri/target/release/bundle/nsis/`。本地交付的可执行文件和安装器可保存在 `release/`。如已清理本地依赖，开发前先运行 `pnpm install --frozen-lockfile`。构建依赖和验收方法见 [Phase 0 验证记录](docs/phase-0.md)。
