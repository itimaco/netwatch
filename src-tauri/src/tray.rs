use crate::{config, icon, popup, probes::proxy, state::{NetState, Snapshot}, AppState};
use parking_lot::Mutex;
use std::{sync::atomic::Ordering::SeqCst, time::Duration};
use tauri::async_runtime::JoinHandle;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

pub const TRAY_ID: &str = "main-tray";
static LAST_ICON: Mutex<Option<NetState>> = Mutex::new(None);
static LAST_SPEED: Mutex<Option<(String, String, NetState)>> = Mutex::new(None);

fn shape(state: NetState) -> icon::Shape { if state == NetState::Unstable { icon::Shape::Ring } else { icon::Shape::Solid } }
fn dpi_scale(app: &AppHandle) -> usize { if cfg!(target_os = "macos") { return 2; } app.primary_monitor().ok().flatten().map(|monitor| monitor.scale_factor().round() as usize).unwrap_or(1).clamp(1, 4) }

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "داشبورد و آمار", true, None::<&str>)?;
    let check = MenuItem::with_id(app, "check", "بررسی مجدد", true, None::<&str>)?;
    let proxy_item = MenuItem::with_id(app, "proxy", "خاموش‌کردن پروکسی سیستم", true, None::<&str>)?;
    let quiet = CheckMenuItem::with_id(app, "quiet", "حالت سکوت", true, false, None::<&str>)?;
    let speed_item = CheckMenuItem::with_id(app, "tray_speed", "نمایش سرعت روی آیکون", true, app.state::<AppState>().settings.read().tray_speed, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "خروج", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&open, &check, &proxy_item, &quiet, &speed_item, &separator, &quit])?;
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon::make([120, 120, 120, 255], icon::Shape::Solid, None))
        .tooltip("NetWatch")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => popup::open_dashboard(app),
            "check" => app.state::<AppState>().kick.notify_one(),
            "proxy" => { let _ = proxy::disable(); app.state::<AppState>().kick.notify_one(); }
            "quiet" => { let state = app.state::<AppState>(); let mut settings = state.settings.write(); settings.quiet = !settings.quiet; let _ = config::save(&state.settings_path.lock(), &settings); }
            "tray_speed" => { let state = app.state::<AppState>(); { let mut settings = state.settings.write(); settings.tray_speed = !settings.tray_speed; let _ = config::save(&state.settings_path.lock(), &settings); } refresh_icon(app); }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            let app = tray.app_handle();
            let rect = match &event { TrayIconEvent::Click { rect, .. } | TrayIconEvent::DoubleClick { rect, .. } | TrayIconEvent::Enter { rect, .. } | TrayIconEvent::Move { rect, .. } | TrayIconEvent::Leave { rect, .. } => Some(rect), _ => None };
            if let Some(rect) = rect { let scale = app.primary_monitor().ok().flatten().map(|monitor| monitor.scale_factor()).unwrap_or(1.0); let position = rect.position.to_logical::<f64>(scale); let size = rect.size.to_logical::<f64>(scale); if size.width > 0.0 { *app.state::<AppState>().tray_rect.lock() = Some((position.x, position.y, size.width, size.height)); } }
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event { popup::open_dashboard(&app); }
        })
        .build(app)?;
    Ok(())
}

pub fn set_speed_icon(app: &AppHandle, rx_bps: u64, tx_bps: u64, state: NetState) {
    let key = (icon::format_mb(rx_bps), icon::format_mb(tx_bps), state);
    { let mut last = LAST_SPEED.lock(); if last.as_ref() == Some(&key) { return; } *last = Some(key.clone()); }
    if let Some(tray) = app.tray_by_id(TRAY_ID) { let _ = tray.set_icon(Some(icon::speed(&key.0, &key.1, state.color(), dpi_scale(app)))); }
    *LAST_ICON.lock() = None;
}

pub fn set_state_icon(app: &AppHandle, state: NetState) {
    let app_state = app.state::<AppState>();
    if app_state.settings.read().tray_speed { let (rx, tx) = { let speed = app_state.speed.lock(); (speed.rx_bps, speed.tx_bps) }; *LAST_SPEED.lock() = None; set_speed_icon(app, rx, tx, state); return; }
    let mut last = LAST_ICON.lock();
    if *last == Some(state) { return; }
    if let Some(tray) = app.tray_by_id(TRAY_ID) { if tray.set_icon(Some(icon::make(state.color(), shape(state), None))).is_ok() { *last = Some(state); } }
}

pub fn refresh_icon(app: &AppHandle) { *LAST_ICON.lock() = None; *LAST_SPEED.lock() = None; let state = app.state::<AppState>().snapshot.lock().state; set_state_icon(app, state); }

pub fn spin(app: &AppHandle) -> Option<JoinHandle<()>> {
    if cfg!(target_os = "linux") || app.state::<AppState>().settings.read().tray_speed { return None; }
    let app = app.clone();
    Some(tauri::async_runtime::spawn(async move { let state = app.state::<AppState>(); let mut index = 0u32; tokio::time::sleep(Duration::from_millis(400)).await; while state.checking.load(SeqCst) { let current = state.snapshot.lock().state; if let Some(tray) = app.tray_by_id(TRAY_ID) { let _ = tray.set_icon(Some(icon::make(current.color(), shape(current), Some(index as f32 * 0.785)))); *LAST_ICON.lock() = None; } index = (index + 1) % 8; tokio::time::sleep(Duration::from_millis(120)).await; } }))
}

pub fn update_tooltip(app: &AppHandle, snapshot: &Snapshot) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else { return };
    let format_ms = |value: Option<u32>| value.map(|ms| format!("{ms}ms")).unwrap_or_else(|| "—".into());
    let mut tip = format!("{} {}\nداخلی: {} | خارجی: {}", snapshot.state.emoji(), snapshot.state.title(), format_ms(snapshot.domestic_ping), format_ms(snapshot.international_ping));
    if let Some(ms) = snapshot.vpn_ping { tip.push_str(&format!(" | تونل: {ms}ms")); }
    if snapshot.loss_pct > 0 { tip.push_str(&format!(" | پکت‌لاس: {}%", snapshot.loss_pct)); }
    { let state = app.state::<AppState>(); let speed = state.speed.lock(); let bytes = state.settings.read().speed_in_bytes; tip.push_str(&format!("\n↓ {}   ↑ {}", crate::speed::fmt(speed.rx_bps, bytes), crate::speed::fmt(speed.tx_bps, bytes))); }
    if snapshot.via_vpn { tip.push_str(&format!("\n🛡️ VPN فعال — خروجی: {}", snapshot.exit.country.clone().unwrap_or_default())); } else if let Some(tun) = &snapshot.tun { tip.push_str(&format!("\nTUN بالا ولی خروجی ایران است: {tun}")); } else if snapshot.proxy.configured { tip.push_str(&format!("\nپروکسی {}:{} {}", snapshot.proxy.host, snapshot.proxy.port, if snapshot.proxy.listening { "روشن ولی کار نمی‌کند ✗" } else { "بسته ✗" })); }
    if let Some(note) = &snapshot.note { tip.push_str(&format!("\n{note}")); }
    let _ = tray.set_tooltip(Some(tip));
}

pub fn update(app: &AppHandle, snapshot: &Snapshot) { set_state_icon(app, snapshot.state); update_tooltip(app, snapshot); }
