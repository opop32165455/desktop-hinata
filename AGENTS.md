# AGENTS.md —— 给 AI 编程助手和后来的开发者

面向用户的说明在 [README.md](README.md)，需求在 [需求.md](需求.md)。这里只写改代码前必须知道、但从代码里不容易看出来的事实。

## 规则

- 不依赖任何第三方宿主（Plash、Wallpaper Engine）或系统脚本（osascript、Apache）。不改写用户的系统壁纸、音量或任何系统设置。
- 应用级设置（隐藏/显示壁纸、可操作/纯壁纸、开机自启动）统一放**托盘菜单**（Windows 右下角托盘图标 / macOS 菜单栏图标），不要放前端设置面板；前端设置面板只放观感与播放类偏好。开机自启动用 `tauri-plugin-autostart`。
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
- 可操作模式：脱离 `WorkerW`，变回 `HWND_BOTTOM` 的顶层窗口。窗口**覆盖整屏，包括任务栏那一条**——Windows 的任务栏可以是透明的，壁纸要能从底下透出来，所以不能只占工作区。
- **任务栏会「消失」的坑**：Windows shell 检测到「覆盖整屏、且处在 Z 序最上层」的窗口时，会摘掉任务栏 `Shell_TrayWnd` 的 `WS_EX_TOPMOST`，任务栏随即被盖住。新建的窗口默认排在 Z 序最前，要等 `apply_mode()` 才被压到 `HWND_BOTTOM`——这个中间态就是竞态的窗口期，表现为「任务栏有时消失」，时好时坏。
- 所以建窗用 `.focused(false).visible(false)`，先 `apply_mode()` 摆好层级再 `show()`；`set_wallpaper_mode()` 里也**不带 `SWP_SHOWWINDOW`**，保证「摆层级」一定早于「变可见」。不抢焦点还让 `show()` 走 `SW_SHOWNOACTIVATE`（tao 的 `MARKER_DONT_FOCUS`，见 `window_state.rs`），窗口不会闪到所有应用前面。
- 另外 `set_always_on_bottom(true)` 会让 tao 在**每次** `WM_WINDOWPOSCHANGING` 时强制 `hwndInsertAfter = HWND_BOTTOM`（`tao/.../windows/event_loop.rs:1139`），所以窗口被压到底之后就再也回不到上层。
- 副作用：窗口启动时不再抢焦点，所以「失焦静音」要等用户点过一次壁纸之后才生效（前端 `nativeActive` 默认 `true`）。
- 若要更保险，可再加 `WS_EX_NOACTIVATE`（Tauri 的 `.focusable(false)`）让窗口永不成为前台窗口；代价是键盘输入和 `app-active` 焦点事件都会失效。
- 这段代码已经在 Windows 上编译并打出 NSIS 安装包，但**没有实际运行验证**过，窗口层级的真实表现待确认。
- 在 Windows 上构建：用仓库根目录的 `build-windows.bat`（自动找 MSVC / Windows SDK 并设好环境），或手动设好 `PATH` / `LIB` / `INCLUDE`。rustc 不会自己配 MSVC 环境，Git Bash 自带的 `link.exe`（coreutils）还会抢占 MSVC 链接器，这两点必须显式处理。
- `windows` crate 的版本要和 Tauri 依赖的一致（现在是 0.62），否则 `window.hwnd()` 返回的 `HWND` 类型对不上。

## 图标

- macOS 26 以后，四角透明的图标会被系统缩小后塞进一块圆角底板。应用图标原图 `docs/app-icon.png` 必须铺满整个 1024 方形，圆角交给系统裁。
- 菜单栏图标 `src-tauri/icons/tray.png` 是 44×44 的黑色剪影加透明通道，以模板图标方式加载，系统按菜单栏深浅自动显示成白或黑。
- Windows 没有模板图标机制（`icon_as_template` 不生效），托盘改用单独的白色剪影 `src-tauri/icons/tray-white.png`——由 `tray.png` 保留 alpha、把 RGB 刷成 255 得到。换托盘图标时两张都要换。
- `npm run icon` 会顺带生成 `icons/android`、`icons/ios`，用不到，删掉即可。
- 换图标后访达和启动台可能还显示旧的，是系统缓存：`killall Dock Finder`。

## 打包时会遇到的事

- 打包失败后可能留下一个挂载着的 `rw.*.dmg`（在 `target/release/bundle/macos/`），下一次 DMG 步骤会失败。先 `hdiutil detach` 再删掉它。
- 安装到 `/Applications` 之前先结束正在运行的 `hinata-desktop` 进程。
- 应用是 `Accessory` 模式：不在 Dock 显示图标，也没有自己的菜单栏。
- Windows 走 NSIS：makensis 要压缩约 1GB 视频素材，这一步约 9 分钟属正常。产物在 `src-tauri/target/release/bundle/nsis/`，约 1.26 GB。
- 只改了代码、没动视频时，用 `build-windows.bat nomedia` 出轻量更新包：`--config src-tauri/tauri.nomedia.conf.json` 用 JSON Merge Patch（RFC 7396）把 `bundle.resources` 置空（数组整体替换），WebView2 也从 `offlineInstaller` 换成 `downloadBootstrapper`，所以又快又小。它**只能装在已装过完整版的机器上**——包里没有 media，视频靠原安装目录里的那份。NSIS 的 `Section Install` 只覆盖自己带的文件、不删其它文件，所以覆盖安装不会动 `media/`；但卸载程序是按当前安装包生成的，装过 nomedia 版之后卸载不再清理 `media/`，会留下约 1GB。
- 两个 flavour 的产物文件名相同（都由 productName + version 决定）。`nomedia` 构建会先把已存在的完整包改名成 `Hinata-full-setup.exe` 保住，构建完再把自己的产物改名成 `Hinata-update-setup.exe`，两者互不覆盖。
- Windows 上偶发 `os error 5 拒绝访问`（写 target 下的 `.d` 文件），疑似杀软瞬时锁，删掉 `src-tauri/target` 重建即可。

## 视频素材

- `tools/build-loop-media.sh` 从 WebM 母版（VP9）一次编码出「正放 + 倒放」合并文件，默认 H.264、CRF 12、每 30 帧一个关键帧（倒带和折返靠频繁 seek）。
- 选 H.264 是为了 macOS（WKWebView）和 Windows（WebView2）都能原生硬解。HEVC 在 Windows 上需要用户另装扩展。
- 母版不在本仓库里，路径写在脚本开头的 `MASTERS`。
- 重新生成后改 `app.js` 的 `MEDIA_VERSION`。
