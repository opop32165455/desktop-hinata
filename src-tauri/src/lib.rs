//! 桌面伴随放置游戏外壳：每块显示器一个无边框、垫在桌面层的全屏窗口，
//! 窗口范围覆盖到 Dock 栏正下方，由前端 CSS 控制底部颜色，Dock 毛玻璃自然跟色。

use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

static HIDDEN: AtomicBool = AtomicBool::new(false);
static INTERACTIVE: AtomicBool = AtomicBool::new(true);

/// 老板键：一键隐藏 / 恢复所有桌面窗口，并通知前端静音。
fn toggle_boss(app: &AppHandle) {
    let hidden = !HIDDEN.load(Ordering::SeqCst);
    HIDDEN.store(hidden, Ordering::SeqCst);
    for (_, w) in app.webview_windows() {
        let _ = if hidden { w.hide() } else { w.show() };
    }
    let _ = app.emit("boss-key", hidden);
}

/// 把窗口放到桌面层（壁纸模式）：压在桌面图标之下，不抢焦点、不进切换器。
/// `geom` = 该显示器的物理坐标与尺寸 (x, y, w, h)。
#[cfg(target_os = "macos")]
fn set_wallpaper_mode(window: &WebviewWindow, _geom: (i32, i32, u32, u32), wallpaper: bool) {
    use objc2::{msg_send, runtime::AnyObject};
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGWindowLevelForKey(key: i32) -> i32;
    }
    // 层级数值随系统版本不同，必须运行时查询：2 = kCGDesktopWindowLevelKey，18 = kCGDesktopIconWindowLevelKey
    let (desktop, icons) = unsafe { (CGWindowLevelForKey(2) as isize, CGWindowLevelForKey(18) as isize) };
    const BEHAVIOR: usize = 1 /* canJoinAllSpaces */ | 16 /* stationary */ | 64 /* ignoresCycle */;
    if let Ok(ptr) = window.ns_window() {
        unsafe {
            let ns: &AnyObject = &*(ptr as *mut AnyObject);
            // 壁纸模式：系统壁纸之上、桌面图标之下。交互模式只比桌面图标层高一级：能接收点击，但仍在所有应用窗口和 Dock 之下，不挡任何东西。
            // 系统在「桌面图标 + 1」层放了一条菜单栏的衬底窗口（WindowServer 的 underbelly，贴着菜单栏下沿的一条亮带）。
            // 交互模式若与它同层，谁在上面是不确定的，亮带就时有时无；所以要再高一级，稳定盖住它。
            let level: isize = if wallpaper { desktop + 1 } else { icons + 2 };
            let _: () = msg_send![ns, setLevel: level];
            let _: () = msg_send![ns, setCollectionBehavior: BEHAVIOR];
        }
    }
}

/// Windows：壁纸模式 = 嵌入桌面图标层之后的 WorkerW 窗口（Wallpaper Engine 同款做法）。
/// 嵌入后鼠标事件会被桌面图标层截走，所以交互模式 = 脱离 WorkerW，变回压在所有窗口之下的顶层窗口。
#[cfg(target_os = "windows")]
fn set_wallpaper_mode(window: &WebviewWindow, geom: (i32, i32, u32, u32), wallpaper: bool) {
    use windows::core::{w, BOOL};
    use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::*;

    unsafe extern "system" fn find_workerw(top: HWND, out: LPARAM) -> BOOL {
        // 带桌面图标（SHELLDLL_DefView）的顶层窗口，它后面紧跟的 WorkerW 就是壁纸层。
        if FindWindowExW(Some(top), None, w!("SHELLDLL_DefView"), None).is_ok() {
            if let Ok(worker) = FindWindowExW(None, Some(top), w!("WorkerW"), None) {
                *(out.0 as *mut HWND) = worker;
            }
        }
        BOOL(1)
    }

    let Ok(hwnd) = window.hwnd() else { return };
    let (x, y, w, h) = geom;
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        if wallpaper {
            let Ok(progman) = FindWindowW(w!("Progman"), None) else { return };
            // 让资源管理器生成 WorkerW 壁纸层
            SendMessageTimeoutW(progman, 0x052C, WPARAM(0xD), LPARAM(0x1), SMTO_NORMAL, 1000, None);
            let mut parent = HWND::default();
            let _ = EnumWindows(Some(find_workerw), LPARAM(&mut parent as *mut HWND as isize));
            // Windows 11 24H2 起壁纸层成了 Progman 的子窗口，找不到 WorkerW 时直接挂 Progman。
            if parent.0.is_null() { parent = progman; }
            SetWindowLongPtrW(hwnd, GWL_STYLE, (style & !(WS_POPUP.0 as isize)) | WS_CHILD.0 as isize);
            let _ = SetParent(hwnd, Some(parent));
            // 子窗口坐标相对虚拟桌面左上角
            let vx = GetSystemMetrics(SM_XVIRTUALSCREEN);
            let vy = GetSystemMetrics(SM_YVIRTUALSCREEN);
            let _ = SetWindowPos(hwnd, None, x - vx, y - vy, w as i32, h as i32, SWP_NOZORDER | SWP_NOACTIVATE | SWP_SHOWWINDOW);
        } else {
            let _ = SetParent(hwnd, None);
            SetWindowLongPtrW(hwnd, GWL_STYLE, (style & !(WS_CHILD.0 as isize)) | WS_POPUP.0 as isize);
            // 放到所有窗口最底下（桌面图标之上）：能点，但不挡其他应用和任务栏。
            let _ = SetWindowPos(hwnd, Some(HWND_BOTTOM), x, y, w as i32, h as i32, SWP_NOACTIVATE | SWP_SHOWWINDOW);
            let _ = window.set_always_on_bottom(true);
        }
    }
}

#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
fn set_wallpaper_mode(window: &WebviewWindow, _geom: (i32, i32, u32, u32), wallpaper: bool) {
    let _ = (window, wallpaper);
    let _ = window.set_always_on_bottom(true);
}

/// 壁纸模式 ⇄ 交互模式。壁纸模式不可点击（在 Windows 上被桌面图标层截走），
/// 需要换装、改设置时切到交互模式，用完切回。
fn toggle_interactive(app: &AppHandle) {
    let interactive = !INTERACTIVE.load(Ordering::SeqCst);
    INTERACTIVE.store(interactive, Ordering::SeqCst);
    apply_mode(app, !interactive);
    let _ = app.emit("interactive-mode", interactive);
}

fn apply_mode(app: &AppHandle, wallpaper: bool) {
    let monitors = app.available_monitors().unwrap_or_default();
    for (i, m) in monitors.iter().enumerate() {
        if let Some(w) = app.get_webview_window(&format!("screen-{i}")) {
            let geom = (m.position().x, m.position().y, m.size().width, m.size().height);
            set_wallpaper_mode(&w, geom, wallpaper);
        }
    }
}

/// 为每块显示器创建一个覆盖整屏（含 Dock 区域）的窗口。
fn create_screen_windows(app: &AppHandle) -> tauri::Result<()> {
    for (i, m) in app.available_monitors()?.iter().enumerate() {
        let w = WebviewWindowBuilder::new(app, format!("screen-{i}"), WebviewUrl::App("index.html".into()))
            .title("Hinata")
            .decorations(false)
            .shadow(false)
            .resizable(false)
            .skip_taskbar(true)
            .build()?;
        w.set_position(PhysicalPosition::new(m.position().x, m.position().y))?;
        w.set_size(PhysicalSize::new(m.size().width, m.size().height))?;
        let handle = app.clone();
        w.on_window_event(move |e| {
            if let WindowEvent::Focused(focused) = e {
                // 任一桌面窗口获得焦点即视为「在桌面」，失焦视为去了别的应用。
                if !HIDDEN.load(Ordering::SeqCst) {
                    let _ = handle.emit("app-active", *focused);
                }
            }
        });
    }
    apply_mode(app, false); // 启动为交互模式，托盘可切回壁纸模式
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        // 只允许运行一个实例：重复启动会叠出两层全屏窗口。
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {}))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        toggle_boss(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory); // 不占 Dock 图标

            create_screen_windows(app.handle())?;

            // 老板键：Cmd/Ctrl + Alt + H
            let mods = if cfg!(target_os = "macos") { Modifiers::SUPER } else { Modifiers::CONTROL } | Modifiers::ALT;
            app.global_shortcut().register(Shortcut::new(Some(mods), Code::KeyH))?;

            // 系统托盘
            let toggle = MenuItem::with_id(app, "toggle", "隐藏 / 显示壁纸", true, None::<&str>)?;
            let interactive = MenuItem::with_id(app, "interactive", "切换：可操作 / 纯壁纸", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&toggle, &interactive, &quit])?;
            TrayIconBuilder::new()
                .icon(tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))?)
                .icon_as_template(true) // 菜单栏里只显示单色剪影，随深浅色自动变白/黑
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "toggle" => toggle_boss(app),
                    "interactive" => toggle_interactive(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("启动失败");
}
