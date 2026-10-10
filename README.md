# Hinata · 桌面伴随壁纸

一张会陪你换装的桌面互动壁纸，独立应用，支持 macOS 和 Windows。

> [!NOTE]
> **原作**由 B 站 UP 主 **[FRAC_LAB](https://space.bilibili.com/7570868)** 开发，版权归原作者所有。
> 本项目是在原作 Mac 版二次开发（`desktop-website`，PURPLE · 片刻）基础上的再次改造，作者与 FRAC_LAB 无关联。
> AI 生成，所有角色均为虚构。

<p align="center"><img src="docs/preview.webp" alt="运行效果" width="100%"></p>

和上一版的区别：**不再需要 Plash、Wallpaper Engine 或任何第三方软件**，也不再改写系统壁纸。
用 [Tauri](https://tauri.app) 打包成一个独立应用，装上就能用。需求与架构见 [需求.md](需求.md)。

## 有什么

- 九套穿搭，可以点选、随机挑，或每隔一段时间自动换一套
- 默认来回往复播放，也可以定格在喜欢的那一秒
- 4K60 画质（H.264，macOS 与 Windows 通用）
- 每块显示器各铺一个全屏窗口，垫在所有应用窗口和 Dock / 任务栏下面，不挡任何东西
- 切到别的应用时自动静音，回来恢复

## 安装

### macOS（12 及以上，目前只有 Apple 芯片版）

1. 打开 `Hinata_0.1.0_aarch64.dmg`，把 Hinata 拖进「应用程序」。
2. 安装包没有签名，第一次打开会被系统拦下：到「系统设置 → 隐私与安全性」点「仍要打开」，或右键应用选「打开」。

不需要另外安装任何东西。

### Windows

1. 运行 `Hinata_0.1.0_x64-setup.exe`，按提示安装，装完从右下角托盘的高跟鞋图标进入。
2. 安装包已经**内嵌 WebView2 运行时**，目标机器不需要联网、也不用预装任何东西。
3. 安装包没有签名，首次运行 SmartScreen 可能提示「未知发布者」，点「仍要运行」即可。

安装包约 1.26 GB，其中约 1 GB 是 `media/` 里的九个视频。

## 怎么用

应用运行时**不在 Dock 里显示图标**，入口在屏幕顶部菜单栏（Windows 是右下角托盘）的高跟鞋图标：

| 菜单项 | 作用 |
| --- | --- |
| 隐藏 / 显示壁纸 | 一键把壁纸藏起来或恢复。快捷键 `Cmd + Option + H`（Windows 为 `Ctrl + Alt + H`） |
| 切换：可操作 / 纯壁纸 | **可操作**（默认）：能点击换装、改设置，会盖住桌面图标。**纯壁纸**：界面全部收起，桌面图标露出来，画面不可点击 |
| 开机自启动 | 勾上后，登录系统时自动打开壁纸。默认关闭。Windows 写在 `HKCU\...\CurrentVersion\Run`，macOS 装 LaunchAgent |
| 退出 | 关闭应用 |

- 设置面板：点右上角的设置按钮打开；点面板外面任意位置、再点一次按钮或点叉都能收起。
- 同一时间只会运行一个 Hinata，重复打开不会叠出第二层。

## 目录

| 路径 | 内容 |
| --- | --- |
| `src/` | 前端（HTML / CSS / JS + 图片）。直接作为 Tauri 的前端目录，没有构建步骤 |
| `media/` | 九个视频，H.264 4K60，共约 1GB。**不在仓库里**（单文件超过 GitHub 的 100MB 限制），需要从 Google Drive 下载后放到这里，见下方「视频文件」。**不能放进 `src/`**，原因见 [AGENTS.md](AGENTS.md) |
| `src-tauri/` | Tauri 外壳（Rust）：窗口、层级、托盘、快捷键 |
| `tools/` | 从 WebM 母版生成视频的脚本（需要 ffmpeg） |
| `docs/app-icon.png` | 应用图标原图（1024×1024，铺满不留透明角） |

## 视频文件

`media/` 下的九个 mp4（约 1GB）没有提交到 Git。请从 Google Drive 下载后整个文件夹放到项目根目录：

> Google Drive 链接：`<待填写>`

没有这些视频，应用和前端预览都无法播放壁纸。

## 开发

需要 [Rust](https://rustup.rs) 和 Node.js。

```bash
npm install
```

```bash
npm run dev
```

```bash
npm run build
```

`npm run build` 的产物在 `src-tauri/target/release/bundle/`（macOS 是 `macos/Hinata.app` 和 `dmg/*.dmg`，Windows 是 `nsis/Hinata_*_x64-setup.exe`）。
Windows 安装包必须在 Windows 上打，Mac 上打不出来。

**在 Windows 上打包**：直接跑仓库根目录的 `build-windows.bat`，它会自动找到 MSVC 与 Windows SDK、设好编译环境，再调用项目里的 tauri CLI。加参数 `check` 可以只检查环境不构建。
手动方式等价于先设好 `PATH` / `LIB` / `INCLUDE` 再跑 `npm run build`——Windows 上 rustc 不会自己配好 MSVC 环境，而且 Git Bash 自带的 `link.exe` 会抢占 MSVC 链接器，这两点必须显式处理。

换应用图标：替换 `docs/app-icon.png` 后运行 `npm run icon`。托盘图标要单独替换：macOS 用 `src-tauri/icons/tray.png`（黑色剪影，系统按菜单栏深浅自动反色），Windows 用 `src-tauri/icons/tray-white.png`（白色剪影，Windows 没有模板图标机制）。

只预览前端（没有外壳，声音始终开启）：在项目根目录运行下面的命令，然后打开 `http://localhost:8000/src/?media=../media/`。

```bash
python3 -m http.server
```

重新生成视频（默认 H.264、CRF 12，母版路径见脚本开头的 `MASTERS`）：

```bash
tools/build-loop-media.sh
```

生成后要改 `src/app.js` 里的 `MEDIA_VERSION`。

## 现状

| 项目 | 状态 |
| --- | --- |
| macOS 应用（Apple 芯片） | 可用，已实际运行 |
| macOS Intel 版 | 未打包 |
| Windows 版 | 已能在 Windows 上编译并打出 NSIS 安装包（内嵌 WebView2）；**尚未在实际机器上运行验证** |
| Dock 取色光晕 | 代码已写，默认关闭，未在实际画面上验证。地址加 `?glow=1` 可打开 |
| 签名与公证 | 未做 |
| Steam 接入、离线收益 | 未做（项目里没有金币等数值系统） |
| Linux | 暂不考虑 |
