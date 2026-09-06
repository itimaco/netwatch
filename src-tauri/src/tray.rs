use crate::{config, icon, popup, probes::proxy, state::{NetState, Snapshot}, AppState};
use parking_lot::Mutex;
use std::{sync::atomic::Ordering::SeqCst, time::Duration};
use tauri::async_runtime::JoinHandle;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

pub const TRAY_ID: &str = "main-tray";
pub const SPEED_TRAY_ID: &str = "speed-tray";
static LAST_ICON: Mutex<Option<NetState>> = Mutex::new(None);
static LAST_SPEED: Mutex<Option<(String, String, NetState)>> = Mutex::new(None);
static LAYOUT_ITEMS: Mutex<Option<Vec<(&'static str, CheckMenuItem<tauri::Wry>)>>> = Mutex::new(None);
const LAYOUTS: &[(&str, &str)] = &[("dual", "دو آیکون: وضعیت + سرعت"), ("badge", "یک آیکون: سرعت روی رنگ وضعیت"), ("title", "متن کنار آیکون (فقط مک)"), ("circle", "فقط دایره وضعیت")];

fn layout(app: &AppHandle) -> String { app.state::<AppState>().settings.read().tray_layout.clone() }
fn shape(state: NetState) -> icon::Shape { if state == NetState::Unstable { icon::Shape::Ring } else { icon::Shape::Solid } }
fn dpi_scale(app: &AppHandle) -> usize { if cfg!(target_os = "macos") { return 2; } app.primary_monitor().ok().flatten().map(|monitor| monitor.scale_factor().round() as usize).unwrap_or(1).clamp(1, 4) }

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "داشبورد و آمار", true, None::<&str>)?;
    let check = MenuItem::with_id(app, "check", "بررسی مجدد", true, None::<&str>)?;
    let proxy_item = MenuItem::with_id(app, "proxy", "خاموش‌کردن پروکسی سیستم", true, None::<&str>)?;
    let quiet = CheckMenuItem::with_id(app, "quiet", "حالت سکوت", true, false, None::<&str>)?;
    let mut items = Vec::new();
    for (id, label) in LAYOUTS { if *id == "title" && !cfg!(target_os = "macos") { continue; } items.push((*id, CheckMenuItem::with_id(app, format!("layout:{id}"), *label, true, layout(app) == *id, None::<&str>)?)); }
    let refs: Vec<&dyn tauri::menu::IsMenuItem<_>> = items.iter().map(|(_, item)| item as &dyn tauri::menu::IsMenuItem<_>).collect();
    let layout_menu = Submenu::with_items(app, "نمایش روی تسک‌بار", true, &refs)?;
    *LAYOUT_ITEMS.lock() = Some(items);
    let quit = MenuItem::with_id(app, "quit", "خروج", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&open, &check, &proxy_item, &quiet, &layout_menu, &separator, &quit])?;
    TrayIconBuilder::with_id(TRAY_ID).icon(icon::make([120,120,120,255], icon::Shape::Solid, None)).tooltip("NetWatch").menu(&menu).show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            if let Some(value) = event.id().as_ref().strip_prefix("layout:") { let state = app.state::<AppState>(); { let mut settings = state.settings.write(); settings.tray_layout = value.into(); let _ = config::save(&state.settings_path.lock(), &settings); } refresh_icon(app); return; }
            match event.id().as_ref() { "open" => popup::open_dashboard(app), "check" => app.state::<AppState>().kick.notify_one(), "proxy" => { let _ = proxy::disable(); app.state::<AppState>().kick.notify_one(); }, "quiet" => { let state = app.state::<AppState>(); let mut settings = state.settings.write(); settings.quiet = !settings.quiet; let _ = config::save(&state.settings_path.lock(), &settings); }, "quit" => app.exit(0), _ => {} }
        })
        .on_tray_icon_event(|tray, event| { let app = tray.app_handle(); if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event { popup::open_dashboard(&app); } })
        .build(app)?;
    Ok(())
}

pub fn ensure_speed_tray(app: &AppHandle) -> tauri::Result<()> {
    let want = layout(app) == "dual";
    let exists = app.tray_by_id(SPEED_TRAY_ID).is_some();
    if want && !exists { TrayIconBuilder::with_id(SPEED_TRAY_ID).icon(icon::speed_text("0.0", "0.0", dpi_scale(app))).tooltip("NetWatch — سرعت").show_menu_on_left_click(false).on_tray_icon_event(|tray, event| { if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event { popup::open_dashboard(tray.app_handle()); } }).build(app)?; }
    else if !want && exists { let _ = app.remove_tray_by_id(SPEED_TRAY_ID); }
    if layout(app) != "title" { if let Some(tray) = app.tray_by_id(TRAY_ID) { let _ = tray.set_title(None::<&str>); } }
    *LAST_SPEED.lock() = None;
    Ok(())
}

pub fn on_speed(app: &AppHandle, rx_bps: u64, tx_bps: u64) {
    let state = app.state::<AppState>().snapshot.lock().state;
    let key = (icon::format_mb(rx_bps), icon::format_mb(tx_bps), state);
    { let mut last = LAST_SPEED.lock(); if last.as_ref() == Some(&key) { return; } *last = Some(key.clone()); }
    match layout(app).as_str() {
        "dual" => if let Some(tray) = app.tray_by_id(SPEED_TRAY_ID) { let _ = tray.set_icon(Some(icon::speed_text(&key.0, &key.1, dpi_scale(app)))); let _ = tray.set_tooltip(Some(format!("↓ {} MB/s   ↑ {} MB/s", key.0, key.1))); },
        "badge" => if let Some(tray) = app.tray_by_id(TRAY_ID) { let _ = tray.set_icon(Some(icon::badge(&key.0, &key.1, state.color(), dpi_scale(app)))); *LAST_ICON.lock() = None; },
        "title" => if let Some(tray) = app.tray_by_id(TRAY_ID) { let _ = tray.set_title(Some(format!("↓{} ↑{}", key.0, key.1))); },
        _ => {}
    }
}

pub fn set_state_icon(app: &AppHandle, state: NetState) {
    if layout(app) == "badge" { let state = app.state::<AppState>(); let (rx, tx) = { let speed = state.speed.lock(); (speed.rx_bps, speed.tx_bps) }; *LAST_SPEED.lock() = None; on_speed(app, rx, tx); return; }
    let mut last = LAST_ICON.lock(); if *last == Some(state) { return; }
    if let Some(tray) = app.tray_by_id(TRAY_ID) { if tray.set_icon(Some(icon::make(state.color(), shape(state), None))).is_ok() { *last = Some(state); } }
}

pub fn refresh_icon(app: &AppHandle) { let _ = ensure_speed_tray(app); *LAST_ICON.lock() = None; *LAST_SPEED.lock() = None; let state = app.state::<AppState>().snapshot.lock().state; set_state_icon(app, state); sync_layout_menu(app); }
fn sync_layout_menu(app: &AppHandle) { let current = layout(app); if let Some(items) = LAYOUT_ITEMS.lock().as_ref() { for (id, item) in items { let _ = item.set_checked(*id == current); } } }

pub fn spin(app: &AppHandle) -> Option<JoinHandle<()>> {
    if cfg!(target_os = "linux") || layout(app) == "badge" { return None; }
    let app = app.clone(); Some(tauri::async_runtime::spawn(async move { let state = app.state::<AppState>(); let mut index = 0u32; tokio::time::sleep(Duration::from_millis(400)).await; while state.checking.load(SeqCst) { let current = state.snapshot.lock().state; if let Some(tray) = app.tray_by_id(TRAY_ID) { let _ = tray.set_icon(Some(icon::make(current.color(), shape(current), Some(index as f32 * 0.785)))); *LAST_ICON.lock() = None; } index = (index + 1) % 8; tokio::time::sleep(Duration::from_millis(120)).await; } }))
}

pub fn update_tooltip(app: &AppHandle, snapshot: &Snapshot) { let Some(tray) = app.tray_by_id(TRAY_ID) else { return }; let format_ms = |value: Option<u32>| value.map(|ms| format!("{ms}ms")).unwrap_or_else(|| "—".into()); let mut tip = format!("{} {}\nداخلی: {} | خارجی: {}", snapshot.state.emoji(), snapshot.state.title(), format_ms(snapshot.domestic_ping), format_ms(snapshot.international_ping)); if let Some(ms) = snapshot.vpn_ping { tip.push_str(&format!(" | تونل: {ms}ms")); } if snapshot.loss_pct > 0 { tip.push_str(&format!(" | پکت‌لاس: {}%", snapshot.loss_pct)); } { let state = app.state::<AppState>(); let speed = state.speed.lock(); tip.push_str(&format!("\n↓ {}   ↑ {}", crate::speed::fmt(speed.rx_bps, true), crate::speed::fmt(speed.tx_bps, true))); } if let Some(note) = &snapshot.note { tip.push_str(&format!("\n{note}")); } let _ = tray.set_tooltip(Some(tip)); }
pub fn update(app: &AppHandle, snapshot: &Snapshot) { set_state_icon(app, snapshot.state); update_tooltip(app, snapshot); }
