use crate::AppState;
use std::sync::atomic::{AtomicU64, Ordering::SeqCst};
use std::time::Duration;
use tauri::{AppHandle, Emitter, LogicalPosition, Manager, WebviewUrl, WebviewWindowBuilder};

const W: f64 = 290.0;
const H: f64 = 210.0;
static GEN: AtomicU64 = AtomicU64::new(0);

fn position(app: &AppHandle) -> (f64, f64) {
    let state = app.state::<AppState>();
    let (monitor_width, monitor_height) = app.primary_monitor().ok().flatten().map(|monitor| { let size = monitor.size().to_logical::<f64>(monitor.scale_factor()); (size.width, size.height) }).unwrap_or((1280.0, 720.0));
    if let Some((x, y, width, height)) = *state.tray_rect.lock() { let center_x = (x + width / 2.0 - W / 2.0).clamp(8.0, monitor_width - W - 8.0); let top = if cfg!(target_os = "macos") { y + height + 6.0 } else { y - H - 8.0 }; return (center_x, top.clamp(8.0, monitor_height - H - 8.0)); }
    if cfg!(target_os = "macos") { (monitor_width - W - 12.0, 36.0) } else { (monitor_width - W - 12.0, monitor_height - H - 56.0) }
}

pub fn payload(app: &AppHandle) -> serde_json::Value { let state = app.state::<AppState>(); let snapshot = state.snapshot.lock().clone(); let speed = state.speed.lock().clone(); serde_json::json!({"snap":snapshot,"title":snapshot.state.title(),"hint":snapshot.state.hint(),"color":snapshot.state.hex(),"rx":crate::speed::fmt(speed.rx_bps,true),"tx":crate::speed::fmt(speed.tx_bps,true)}) }
pub fn schedule_show(app: &AppHandle) { let generation = GEN.fetch_add(1, SeqCst) + 1; let app = app.clone(); tauri::async_runtime::spawn(async move { tokio::time::sleep(Duration::from_millis(350)).await; if GEN.load(SeqCst) != generation { return; } let (x, y) = position(&app); let window = match app.get_webview_window("hover") { Some(window) => { let _ = window.set_position(LogicalPosition::new(x, y)); window }, None => match WebviewWindowBuilder::new(&app, "hover", WebviewUrl::App("hover.html".into())).title("NetWatch").decorations(false).transparent(true).always_on_top(true).skip_taskbar(true).resizable(false).focused(false).shadow(false).visible(false).inner_size(W, H).position(x, y).build() { Ok(window) => { tokio::time::sleep(Duration::from_millis(200)).await; window }, Err(_) => return } }; if GEN.load(SeqCst) != generation { return; } let _ = window.emit("hover-data", payload(&app)); let _ = window.show(); }); }
pub fn schedule_hide(app: &AppHandle) { GEN.fetch_add(1, SeqCst); let app = app.clone(); tauri::async_runtime::spawn(async move { tokio::time::sleep(Duration::from_millis(200)).await; if let Some(window) = app.get_webview_window("hover") { let _ = window.hide(); } }); }
pub fn refresh(app: &AppHandle) { if let Some(window) = app.get_webview_window("hover") { if window.is_visible().unwrap_or(false) { let _ = window.emit("hover-data", payload(app)); } } }
