# Blis Terminal

面向 Windows 10/11 x64 的个人命令终端。启动后在 `blis>` 提示符输入 `help`，可查看当前的内置命令。使用上下方向键浏览本次会话的输入历史。

每条命令都可以用 `help <命令>`、`<命令> --help` 或 `<命令> -h` 查看详细用法和示例。命令的说明与执行函数一起注册，命令列表和详细帮助从同一份定义生成。

输入 `shortcut` 可查看快捷键。输入命令名的前几个字母后按 `Tab` 补全，例如 `se` 补成 `search`。有多个匹配时连续按 `Tab` 可切换。`Ctrl+L` 清屏，`Ctrl+O` 折叠或展开已执行命令的输出。可用鼠标拖选屏幕上的命令或 `history` 输出，再按 `Ctrl+C` 复制。上下方向键可取回历史命令继续编辑。

新增命令时实现 `CommandDefinition`，再加入 `apps/desktop/src/terminal-commands.ts` 的命令表。

## 搜索与打开网页

```text
search Rust 入门教程
search bilibili csgo
search -s github "vue terminal"
search --site zhihu -b chrome "Windows 托盘"
search https://github.com/liyupi/yuindex

search browser list
search browser default chrome
search browser add edge --app "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe"
search browser default system
search browser remove edge

search site list
search site add demo -t "https://example.com/search?q={query}"
search site remove demo
```

省略站点时使用百度。预置百度、Bing、Google、GitHub、Bilibili 和知乎。自定义站点与浏览器别名不区分英文字母大小写。`-s` 与 `--site` 选站点，`-b` 与 `--browser` 临时选浏览器。`--` 之后按字面搜索，例如 `search -- bilibili` 会在百度搜索 bilibili。

Blis 初始跟随 Windows 默认浏览器，并自动登记安装在标准位置的 Chrome。`search browser default chrome` 将 Chrome 设为 Blis 内默认。`search site add` 使用 `-t` 或 `--template` 指定 HTTP(S) 模板，查询参数中需要有一个完整的 `{query}` 值。自定义设置保存在 `%APPDATA%/dev.blis.terminal/search-settings.json`，重启后可继续使用。输入 `help search`、`search site --help` 或 `search browser --help` 查看说明。

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
