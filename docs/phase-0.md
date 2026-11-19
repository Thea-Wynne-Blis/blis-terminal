# Phase 0 桌面验证

## 架构决策

- 使用 Tauri 2、Vue 3、TypeScript 和 pnpm Workspace。Rust 管理后台进程、单实例、全局快捷键和窗口生命周期。单实例插件优先注册。
- 手动启动时打开终端。带 `--background` 启动时先初始化托盘和快捷键，首次唤醒再创建 WebView2 窗口。之后隐藏并复用窗口，方便验证输入能否保留，也便于比较冷启动与唤醒后后台的内存。
- 默认热键 `Alt+Space` 注册失败时尝试 `Alt+Shift+Space`。窗口显示实际注册结果。自定义热键设置放在后续阶段。
- 前端仅获得读取已注册快捷键和隐藏当前窗口的权限。发布页面启用 CSP，安装器使用 NSIS 和系统 WebView2 Runtime。

## 验收步骤

1. 运行 `pnpm tauri build -- --locked`。验证出现 `apps/desktop/src-tauri/target/release/blis-terminal.exe` 和 `apps/desktop/src-tauri/target/release/bundle/nsis/*-setup.exe`。
2. 关闭此前运行的 Blis Terminal。在 PowerShell 中执行下方启动命令，确认托盘出现。记录冷启动后台的进程树内存。
3. 按全局快捷键，确认窗口显示，能够立即输入。等待 10 秒后记录打开状态的内存。
4. 输入一段文字并按 `Esc`，确认窗口隐藏，且进程和托盘仍在。用热键唤醒，确认文字仍在且可继续输入。关闭窗口，再确认同样可以唤醒。
5. 窗口隐藏期间再次运行同一可执行文件。确认新进程退出、原 PID 仍在、原窗口获得焦点。再隐藏窗口，等待 10 秒并记录唤醒后后台的内存。
6. 托盘菜单选择「退出」，确认原进程结束。在 Windows 10 和 Windows 11 分别重复验证。

```powershell
$exe = 'apps/desktop/src-tauri/target/release/blis-terminal.exe'
$app = Start-Process $exe -ArgumentList '--background' -PassThru
Start-Sleep -Seconds 10
./scripts/Measure-TerminalMemory.ps1 -ProcessId $app.Id
Get-ChildItem apps/desktop/src-tauri/target/release/blis-terminal.exe,
  apps/desktop/src-tauri/target/release/bundle/nsis/*-setup.exe |
  Select-Object Name, Length
```

每个内存状态采样 5 次，间隔 1 秒，报告中位数。Working Set 为进程当前驻留内存，进程树求和可能重复计算共享页。Private 指 Private Bytes，为进程独占提交的虚拟内存。记录系统版本、WebView2 版本以及 CPU 和 RAM。若某个进程的父进程不属于应用进程树，记录这一口径差异。

## 2026-11-19 首次实测（调整默认窗口尺寸前）

环境为 Windows 11 25H2 x64（26200.9550）、WebView2 154.0.4258.53、Intel Core i7-13620H、可见物理内存 15.7 GiB。使用 Tauri 2.12.6、Rust 1.100.0 和 Node.js 24.16.0 构建 Release。

| 状态                 | WebView2 进程数 | 主进程 Working Set / Private Bytes | WebView2 Working Set / Private Bytes | 整棵进程树 Working Set / Private Bytes |
| -------------------- | --------------: | ---------------------------------: | -----------------------------------: | -------------------------------------: |
| 冷启动后台，尚未唤醒 |               0 |                   12.86 / 2.12 MiB |                            0 / 0 MiB |                       12.86 / 2.12 MiB |
| 终端窗口打开         |               6 |                   26.86 / 5.36 MiB |                  402.12 / 190.58 MiB |                    428.94 / 195.91 MiB |
| 唤醒后隐藏到后台     |               6 |                   27.15 / 5.43 MiB |                  410.64 / 195.81 MiB |                    437.79 / 201.24 MiB |

`blis-terminal.exe` 为 8,993,792 字节（8.58 MiB），NSIS 安装器为 1,997,601 字节（1.91 MiB）。安装器使用系统 WebView2，缺失时由 Tauri 的默认引导程序下载。

实测确认 `--background` 启动时未创建终端窗口，第二次运行会退出并唤醒原实例。`Alt+Space` 重新显示同一个窗口并恢复输入焦点。模拟默认快捷键冲突时，验证了 `Alt+Shift+Space` 可以唤醒窗口。`Esc` 与窗口关闭按钮均能隐藏窗口，原进程继续运行。Release 页面验证了实际快捷键的显示，以及重新唤醒后的草稿保留。Windows 10、托盘菜单退出、安装和卸载流程、唤醒 P95 待验证。

后台窗口在至少一次唤醒后保留 WebView2，进程树内存高于设计稿的后台和窗口打开目标。后续可验证隐藏后按空闲时间释放 WebView，并在会话层保留草稿和输出，再比较内存与唤醒延迟。

## 自适应窗口尺寸验证

在可用区域为 2560 × 1368 像素的 Windows 11 显示器上，首次启动窗口为 1280 × 862 像素，居中。验证了原生缩放边框和最大化按钮。将窗口调整为 1400 × 912 像素后，关闭至托盘再唤醒，窗口和尺寸均保留。多显示器混合 DPI 场景待实机验证。

同一 Release 验证了 `blis>` 输入焦点、命令输出、未知命令提示、上下键历史和清屏。可执行文件为 8.76 MiB，NSIS 安装器为 1.94 MiB。
