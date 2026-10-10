# AGENTS.md —— 给 AI 编程助手和后来的开发者

面向用户的说明在 [README.md](README.md)，需求在 [需求.md](需求.md)。这里只写改代码前必须知道、但从代码里不容易看出来的事实。

## 规则

- 不依赖任何第三方宿主（Plash、Wallpaper Engine）或系统脚本（osascript、Apache）。不改写用户的系统壁纸、音量或任何系统设置。
- 应用启动后会铺满全屏。窗口层级改错会盖住用户的所有应用和 Dock，动层级之前先读下面「窗口层级」一节，改完用 `CGWindowListCopyWindowInfo` 查实际层级再让用户试。
- 原作归 B 站 FRAC_LAB 所有，界面里的署名和 README 的声明不要删。

## 结构

- `src/`：纯静态前端，没有打包器。`app.js` 是一个大的 IIFE，`catalog.js` 提供穿搭数据（`LOOKS`）。
- `src-tauri/src/lib.rs`：全部原生逻辑，一个文件。
- 前端和外壳之间只有三个事件（外壳 → 前端），在 `app.js` 的 `initNativeBridge` 里接收：
  - `app-active`（bool）：窗口得到 / 失去焦点，用于离开时静音
  - `boss-key`（bool，true = 已隐藏）
  - `interactive-mode`（bool）：true = 可操作，false = 纯壁纸
- 前端靠 `<html>` 上的两个类区分状态：`is-native`（在 Tauri 里）、`is-interactive`（可操作模式）。纯壁纸模式的界面隐藏规则在 `style.css` 末尾。

## 视频为什么不在 `src/`

Tauri 会把 `frontendDist`（即 `src/`）整个压缩后嵌进可执行文件。1GB 视频放进去，编译进程会被系统直接杀掉（SIGKILL）。
所以视频放在根目录 `media/`，通过 `tauri.conf.json` 的 `bundle.resources` 打进应用的资源目录，运行时经 asset 协议读取：

- `app.security.assetProtocol` 已开启，范围 `$RESOURCE/media/**`；Cargo 里要有 `protocol-asset` 特性。
- `app.js` 的 `resolveNativeMedia()` 启动时把九个文件的路径换成 asset 地址，之后才开始播放。
- asset 地址和页面不同源：对视频 `drawImage` 后 canvas 会被污染，`getImageData` 抛错。Dock 光晕因此退回取定格图的颜色。

## 窗口层级（macOS）

层级数值随系统版本不同，**不能写死**，必须运行时用 `CGWindowLevelForKey` 查（2 = 桌面，18 = 桌面图标）。
这台开发机（Darwin 27）上桌面是 `i32::MIN + 25`，不是常见资料里的 `+ 20`。

| 模式 | 层级 | 原因 |
| --- | --- | --- |
| 纯壁纸 | 桌面 + 1 | 系统壁纸之上、桌面图标之下 |
| 可操作 | 桌面图标 + 2 | 能接收点击，仍在所有应用窗口（0）和 Dock（20）之下 |

可操作模式必须是「图标 + 2」而不是「+ 1」：WindowServer 在「图标 + 1」放了一个名为 `underbelly` 的窗口（菜单栏下沿一条约 91px 高的亮带）。
和它同层时上下顺序不固定，表现为菜单栏下方一串时有时无的白光，而且截图、录屏拍不到。

不要用 `NSFloatingWindowLevel` 之类高于 0 的层级：全屏窗口会挡住所有应用和 Dock。

## Windows

- 纯壁纸模式：把窗口 `SetParent` 到桌面图标层后面的 `WorkerW`（找不到时挂到 `Progman`，对应 Windows 11 24H2）。嵌入后鼠标事件被图标层截走，所以不可点击。
- 可操作模式：脱离 `WorkerW`，变回 `HWND_BOTTOM` 的顶层窗口。
- 这段代码只在 Mac 上用 `cargo check --target x86_64-pc-windows-msvc` 对单独抽出的函数做过类型检查，完整工程没在 Windows 上编译过（构建脚本需要 Windows 的资源编译器）。
- `windows` crate 的版本要和 Tauri 依赖的一致（现在是 0.62），否则 `window.hwnd()` 返回的 `HWND` 类型对不上。

## 图标

- macOS 26 以后，四角透明的图标会被系统缩小后塞进一块圆角底板。应用图标原图 `docs/app-icon.png` 必须铺满整个 1024 方形，圆角交给系统裁。
- 菜单栏图标 `src-tauri/icons/tray.png` 是 44×44 的黑色剪影加透明通道，以模板图标方式加载，系统按菜单栏深浅自动显示成白或黑。
- `npm run icon` 会顺带生成 `icons/android`、`icons/ios`，用不到，删掉即可。
- 换图标后访达和启动台可能还显示旧的，是系统缓存：`killall Dock Finder`。

## 打包时会遇到的事

- 打包失败后可能留下一个挂载着的 `rw.*.dmg`（在 `target/release/bundle/macos/`），下一次 DMG 步骤会失败。先 `hdiutil detach` 再删掉它。
- 安装到 `/Applications` 之前先结束正在运行的 `hinata-desktop` 进程。
- 应用是 `Accessory` 模式：不在 Dock 显示图标，也没有自己的菜单栏。

## 视频素材

- `tools/build-loop-media.sh` 从 WebM 母版（VP9）一次编码出「正放 + 倒放」合并文件，默认 H.264、CRF 12、每 30 帧一个关键帧（倒带和折返靠频繁 seek）。
- 选 H.264 是为了 macOS（WKWebView）和 Windows（WebView2）都能原生硬解。HEVC 在 Windows 上需要用户另装扩展。
- 母版不在本仓库里，路径写在脚本开头的 `MASTERS`。
- 重新生成后改 `app.js` 的 `MEDIA_VERSION`。
