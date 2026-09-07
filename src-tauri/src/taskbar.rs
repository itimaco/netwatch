#![cfg(windows)]
use crate::AppState;
use futures::FutureExt;
use parking_lot::Mutex;
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, AtomicU64, Ordering::SeqCst};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use tokio::sync::Notify;
use windows_sys::Win32::Foundation::{BOOL, HWND, RECT};
use windows_sys::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DwmSetWindowAttribute, DWMWA_CLOAK, DWMWA_CLOAKED};
use windows_sys::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST};
use windows_sys::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
use windows_sys::Win32::UI::HiDpi::GetDpiForWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

pub const LABEL: &str = "widget";
const W: f64 = 72.0;
const GAP: f64 = 6.0;
pub static RECT_L: Mutex<Option<(f64, f64, f64, f64)>> = Mutex::new(None);

static WAKE: OnceLock<Arc<Notify>> = OnceLock::new();
static START: OnceLock<Instant> = OnceLock::new();
static SELF_HWND: AtomicIsize = AtomicIsize::new(0);
static SHELL_PID: AtomicU32 = AtomicU32::new(0);
static BUILDING: AtomicBool = AtomicBool::new(false);
static REBUILD_REQ: AtomicBool = AtomicBool::new(false);
static ALIVE_MS: AtomicU64 = AtomicU64::new(0);
static PAGE_VISIBLE: AtomicBool = AtomicBool::new(true);
static PAGE_HIDDEN_SINCE: AtomicU64 = AtomicU64::new(0);

fn wake() -> Arc<Notify> { WAKE.get_or_init(|| Arc::new(Notify::new())).clone() }
fn now_ms() -> u64 { START.get_or_init(Instant::now).elapsed().as_millis() as u64 }
fn wide(s: &str) -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() }
fn rect(h: HWND) -> Option<RECT> { let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 }; (unsafe { GetWindowRect(h, &mut r) } != 0).then_some(r) }
fn hwnd_of(w: &tauri::WebviewWindow) -> Option<HWND> { w.hwnd().ok().map(|h| h.0 as isize as HWND) }
fn monitor(h: HWND) -> Option<MONITORINFO> {
    unsafe {
        let m = MonitorFromWindow(h, MONITOR_DEFAULTTONEAREST);
        let mut mi: MONITORINFO = std::mem::zeroed(); mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        (GetMonitorInfoW(m, &mut mi) != 0).then_some(mi)
    }
}

pub fn alive(visible: bool) {
    let t = now_ms();
    ALIVE_MS.store(t, SeqCst);
    let was = PAGE_VISIBLE.swap(visible, SeqCst);
    if visible { PAGE_HIDDEN_SINCE.store(0, SeqCst); } else if was || PAGE_HIDDEN_SINCE.load(SeqCst) == 0 { PAGE_HIDDEN_SINCE.store(t, SeqCst); }
}
pub fn rebuild(_app: &AppHandle) { REBUILD_REQ.store(true, SeqCst); wake().notify_one(); }

struct Bar { hwnd: HWND, pid: u32, rc: RECT, tray: RECT, scale: f64 }
fn taskbar() -> Option<Bar> {
    unsafe {
        let hwnd = FindWindowW(wide("Shell_TrayWnd").as_ptr(), std::ptr::null());
        if hwnd.is_null() { return None; }
        let mut pid = 0u32; GetWindowThreadProcessId(hwnd, &mut pid);
        let notify = FindWindowExW(hwnd, std::ptr::null_mut(), wide("TrayNotifyWnd").as_ptr(), std::ptr::null());
        let rc = rect(hwnd)?;
        let tray = if notify.is_null() { RECT { left: rc.right - 1, ..rc } } else { rect(notify)? };
        let dpi = GetDpiForWindow(hwnd);
        Some(Bar { hwnd, pid, rc, tray, scale: if dpi == 0 { 1.0 } else { dpi as f64 / 96.0 } })
    }
}

fn fullscreen_active(shell_pid: u32) -> bool {
    unsafe {
        let fg = GetForegroundWindow(); if fg.is_null() { return false; }
        let mut pid = 0u32; GetWindowThreadProcessId(fg, &mut pid);
        if pid == shell_pid || pid == std::process::id() { return false; }
        let ex = GetWindowLongPtrW(fg, GWL_EXSTYLE) as u32;
        if ex & (WS_EX_LAYERED | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW) != 0 { return false; }
        let mut buf = [0u16; 64]; let n = GetClassNameW(fg, buf.as_mut_ptr(), 64) as usize;
        let cls = String::from_utf16_lossy(&buf[..n]);
        if cls.contains("CoreWindow") || cls.contains("Xaml") { return false; }
        let (Some(r), Some(mi)) = (rect(fg), monitor(fg)) else { return false };
        let m = mi.rcMonitor;
        r.left <= m.left && r.top <= m.top && r.right >= m.right && r.bottom >= m.bottom
    }
}

fn style(h: HWND) {
    unsafe {
        SetWindowLongPtrW(h, GWLP_HWNDPARENT, 0);
        let ex = GetWindowLongPtrW(h, GWL_EXSTYLE);
        SetWindowLongPtrW(h, GWL_EXSTYLE, (ex | WS_EX_TOOLWINDOW as isize | WS_EX_NOACTIVATE as isize) & !(WS_EX_APPWINDOW as isize));
        SetWindowPos(h, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED | SWP_NOOWNERZORDER);
    }
}
fn below(h: HWND, bar: HWND) -> bool {
    unsafe {
        let mut w = GetTopWindow(std::ptr::null_mut());
        while !w.is_null() { if w == h { return false; } if w == bar { return true; } w = GetWindow(w, GW_HWNDNEXT); }
        true
    }
}
fn cloaked(h: HWND) -> bool {
    let mut v: u32 = 0;
    unsafe { DwmGetWindowAttribute(h, DWMWA_CLOAKED as u32, &mut v as *mut _ as *mut _, 4) };
    v != 0
}

fn build_when_free(app: &AppHandle) {
    if BUILDING.swap(true, SeqCst) { return; }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        for _ in 0..100 { if app.get_webview_window(LABEL).is_none() { break; } tokio::time::sleep(Duration::from_millis(20)).await; }
        let a2 = app.clone();
        let r = app.run_on_main_thread(move || {
            match WebviewWindowBuilder::new(&a2, LABEL, WebviewUrl::App("widget.html".into()))
                .title("NetWatch").decorations(false).transparent(true).always_on_top(true).skip_taskbar(true)
                .resizable(false).focused(false).shadow(false).visible(false).inner_size(W, 40.0).build()
            { Ok(_) => log::info!("widget: built"), Err(e) => log::warn!("widget: build failed: {e}") }
            ALIVE_MS.store(now_ms(), SeqCst); PAGE_VISIBLE.store(true, SeqCst); PAGE_HIDDEN_SINCE.store(0, SeqCst);
            BUILDING.store(false, SeqCst);
            wake().notify_one();
        });
        if r.is_err() { BUILDING.store(false, SeqCst); }
    });
}

pub fn ensure(app: &AppHandle, want: bool) {
    let existing = app.get_webview_window(LABEL);
    if !want { if let Some(w) = existing { let _ = w.destroy(); } SELF_HWND.store(0, SeqCst); *RECT_L.lock() = None; return; }
    if let Some(w) = &existing {
        if hwnd_of(w).map_or(false, |h| unsafe { IsWindow(h) } != 0) { return; }
        let _ = w.destroy();
    }
    build_when_free(app);
}

unsafe extern "system" fn on_event(_: HWINEVENTHOOK, ev: u32, hwnd: HWND, obj: i32, child: i32, _: u32, _: u32) {
    if obj != 0 || child != 0 { return; }
    let me = SELF_HWND.load(SeqCst);
    let hit = match ev {
        EVENT_SYSTEM_FOREGROUND => true,
        EVENT_OBJECT_HIDE => hwnd as isize == me,
        EVENT_OBJECT_REORDER => hwnd == GetDesktopWindow(),
        EVENT_OBJECT_SHOW => { let mut pid = 0u32; GetWindowThreadProcessId(hwnd, &mut pid); pid == SHELL_PID.load(SeqCst) }
        _ => false,
    };
    if hit { if let Some(n) = WAKE.get() { n.notify_one(); } }
}
fn spawn_hook() {
    std::thread::Builder::new().name("winevent".into()).spawn(|| unsafe {
        let f = WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNTHREAD;
        let _a = SetWinEventHook(EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND, std::ptr::null_mut(), Some(on_event), 0, 0, f);
        let _b = SetWinEventHook(EVENT_OBJECT_SHOW, EVENT_OBJECT_REORDER, std::ptr::null_mut(), Some(on_event), 0, 0, f);
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 { TranslateMessage(&msg); DispatchMessageW(&msg); }
    }).ok();
}

pub fn spawn_placer(app: AppHandle) {
    spawn_hook();
    tauri::async_runtime::spawn(async move {
        loop {
            let r = AssertUnwindSafe(placer_loop(app.clone())).catch_unwind().await;
            log::error!("widget: placer crashed ({}), restarting", r.err().map(|_| "panic").unwrap_or("exit"));
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });
}

async fn placer_loop(app: AppHandle) {
    let n = wake();
    let mut styled: isize = 0;
    let mut shown = false;
    let mut expected: Option<(i32, i32, i32, i32)> = None;
    let mut parked = false;
    let mut fs_ticks = 0u8;
    let mut cloak_ticks = 0u8;
    let mut last_rebuild = Instant::now() - Duration::from_secs(60);

    loop {
        tokio::select! { _ = tokio::time::sleep(Duration::from_millis(400)) => {}, _ = n.notified() => {} }
        tokio::time::sleep(Duration::from_millis(30)).await;

        let state = app.state::<AppState>();
        let (want, hide_fs) = { let s = state.settings.read(); (s.tray_layout == "widget", s.widget_hide_fullscreen) };
        if !want { continue; }

        let Some(w) = app.get_webview_window(LABEL) else { ensure(&app, true); continue };
        let Some(h) = hwnd_of(&w) else { continue };
        if unsafe { IsWindow(h) } == 0 { log::warn!("widget: hwnd gone -> rebuild"); ensure(&app, true); continue; }
        let Some(b) = taskbar() else { continue };
        SHELL_PID.store(b.pid, SeqCst);
        if h as isize != styled { style(h); styled = h as isize; SELF_HWND.store(styled, SeqCst); shown = false; expected = None; parked = false; }

        fs_ticks = if hide_fs && fullscreen_active(b.pid) { fs_ticks.saturating_add(1) } else { 0 };
        if fs_ticks >= 3 {
            if !parked { unsafe { SetWindowPos(h, std::ptr::null_mut(), -32000, -32000, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE); } parked = true; expected = None; }
            continue;
        }
        parked = false;

        let s = b.scale;
        let horiz = (b.rc.right - b.rc.left) > (b.rc.bottom - b.rc.top);
        let bar_h = if horiz { b.rc.bottom - b.rc.top } else { b.rc.right - b.rc.left };
        let hgt = (bar_h - (6.0 * s) as i32).clamp((24.0 * s) as i32, (40.0 * s) as i32);
        let wid = (W * s) as i32;
        let (x, y) = if horiz { (b.tray.left - wid - (GAP * s) as i32, b.rc.top + (bar_h - hgt) / 2) }
                     else { (b.rc.left + (bar_h - wid) / 2, b.tray.top - hgt - (GAP * s) as i32) };
        let key = (x, y, wid, hgt);

        if !shown { let _ = w.show(); shown = true; }
        let mut reason: Option<&'static str> = None;
        if unsafe { IsWindowVisible(h) } == 0 { log::warn!("widget: hidden externally -> show"); let _ = w.show(); }
        if unsafe { IsIconic(h) } != 0 { log::warn!("widget: minimized externally -> restore"); unsafe { ShowWindow(h, SW_SHOWNOACTIVATE); } }

        let actual = rect(h).map(|r| (r.left, r.top, r.right - r.left, r.bottom - r.top));
        let moved = actual.map_or(true, |a| (a.0 - x).abs() > 2 || (a.1 - y).abs() > 2 || (a.2 - wid).abs() > 2 || (a.3 - hgt).abs() > 2);
        if expected != Some(key) || moved {
            if expected == Some(key) { log::warn!("widget: moved/resized externally {:?} -> restore", actual); }
            unsafe { SetWindowPos(h, HWND_TOPMOST, x, y, wid, hgt, SWP_NOACTIVATE | SWP_NOOWNERZORDER); }
            expected = Some(key);
            *RECT_L.lock() = Some((x as f64 / s, y as f64 / s, wid as f64 / s, hgt as f64 / s));
            let _ = w.emit("widget-size", serde_json::json!({ "w": wid as f64 / s, "h": hgt as f64 / s }));
        } else if below(h, b.hwnd) {
            unsafe { SetWindowPos(h, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER); }
        }

        if cloaked(h) {
            cloak_ticks = cloak_ticks.saturating_add(1);
            let off: BOOL = 0;
            unsafe { DwmSetWindowAttribute(h, DWMWA_CLOAK as u32, &off as *const _ as *const _, 4) };
            if cloak_ticks >= 3 { reason = Some("cloaked by DWM"); }
        } else { cloak_ticks = 0; }

        let t = now_ms();
        if ALIVE_MS.load(SeqCst) + 6000 < t { reason = Some("page not responding"); }
        let hs = PAGE_HIDDEN_SINCE.load(SeqCst);
        if !PAGE_VISIBLE.load(SeqCst) && hs != 0 && hs + 3000 < t { reason = Some("webview reports hidden while window shown"); }
        if REBUILD_REQ.swap(false, SeqCst) { reason = Some("manual"); last_rebuild = Instant::now() - Duration::from_secs(60); }

        if let Some(r) = reason {
            if last_rebuild.elapsed() > Duration::from_secs(15) {
                log::warn!("widget: rebuilding ({r})");
                last_rebuild = Instant::now();
                let _ = w.destroy();
                build_when_free(&app);
                styled = 0; shown = false; expected = None; cloak_ticks = 0;
            }
        }
    }
}

pub fn popup_menu(app: &AppHandle) {
    let Some(w) = app.get_webview_window(LABEL) else { return };
    if let Ok(h) = w.hwnd() { unsafe { SetForegroundWindow(h.0 as isize as HWND); } }
    if let Some(m) = crate::tray::menu() { let _ = w.popup_menu(&m); }
}
pub fn hover(app: &AppHandle, on: bool) {
    if on {
        if let Some(r) = *RECT_L.lock() { *app.state::<AppState>().tray_rect.lock() = Some(r); }
        crate::hover::schedule_show(app);
    } else { crate::hover::schedule_hide(app); }
}
