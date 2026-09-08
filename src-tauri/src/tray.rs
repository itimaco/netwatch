use crate::{config, icon, popup, probes::proxy, state::{NetState, Snapshot}, AppState};
use parking_lot::Mutex;
use std::{sync::atomic::Ordering::SeqCst, time::Duration};
use tauri::async_runtime::JoinHandle;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

pub const TRAY_ID: &str = "main-tray";
pub const SPEED_TRAY_ID: &str = "speed-tray";
pub const DOWN_TRAY_ID: &str = "down-tray";
pub const UP_TRAY_ID: &str = "up-tray";
static LAST_ICON: Mutex<Option<NetState>> = Mutex::new(None);
static LAST_SPEED: Mutex<Option<(String, String, NetState)>> = Mutex::new(None);
static LAST_DOWN: Mutex<Option<String>> = Mutex::new(None);
static LAST_UP: Mutex<Option<String>> = Mutex::new(None);
static LAYOUT_ITEMS: Mutex<Option<Vec<(&'static str, CheckMenuItem<tauri::Wry>)>>> = Mutex::new(None);
static TOGGLE_ITEMS: Mutex<Option<(CheckMenuItem<tauri::Wry>, CheckMenuItem<tauri::Wry>)>> = Mutex::new(None);
static MENU: Mutex<Option<Menu<tauri::Wry>>> = Mutex::new(None);
const LAYOUTS: &[(&str, &str)] = &[("widget", "ویجت روی تسک‌بار — پیشنهادی (ویندوز)"), ("separate", "آیکون‌های جدا: وضعیت + دانلود + آپلود"), ("dual", "دو آیکون: وضعیت + سرعت (ترکیبی)"), ("badge", "یک آیکون: سرعت روی رنگ وضعیت"), ("title", "متن کنار آیکون (فقط مک)"), ("circle", "فقط دایره وضعیت")];

fn layout(app: &AppHandle) -> String { app.state::<AppState>().settings.read().tray_layout.clone() }
fn shape(state: NetState) -> icon::Shape { if state == NetState::Unstable { icon::Shape::Ring } else { icon::Shape::Solid } }
fn dpi_scale(app: &AppHandle) -> usize { if cfg!(target_os = "macos") { return 2; } app.primary_monitor().ok().flatten().map(|monitor| monitor.scale_factor().round() as usize).unwrap_or(1).clamp(1, 4) }
pub fn on_main(app: &AppHandle, f: impl FnOnce(&AppHandle) + Send + 'static) { let a = app.clone(); let _ = app.run_on_main_thread(move || f(&a)); }
const RLM: &str = "\u{200F}";
fn ltr(v: impl std::fmt::Display) -> String { format!("\u{2066}{v}\u{2069}") }
pub fn menu() -> Option<Menu<tauri::Wry>> { MENU.lock().clone() }

fn build_aux_tray(app: &AppHandle, id: &str, img: tauri::image::Image<'static>, tip: &str) -> tauri::Result<()> {
    TrayIconBuilder::with_id(id)
        .icon(img)
        .tooltip(tip)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            let app = tray.app_handle();
            match event {
                TrayIconEvent::Enter { .. } => { popup::close(&app); crate::hover::schedule_show(&app); },
                TrayIconEvent::Leave { .. } => crate::hover::schedule_hide(&app),
                TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } => { popup::close(&app); crate::hover::schedule_hide(&app); popup::open_dashboard(&app); },
                _ => {}
            }
        })
        .build(app)?;
    Ok(())
}

fn sync_tray(app: &AppHandle, id: &str, want: bool, make: impl FnOnce() -> tauri::image::Image<'static>, tip: &str) -> tauri::Result<()> {
    let exists = app.tray_by_id(id).is_some();
    if want && !exists { build_aux_tray(app, id, make(), tip)?; }
    else if !want && exists { let _ = app.remove_tray_by_id(id); }
    Ok(())
}

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "داشبورد و آمار", true, None::<&str>)?;
    let check = MenuItem::with_id(app, "check", "بررسی مجدد", true, None::<&str>)?;
    let proxy_item = MenuItem::with_id(app, "proxy", "خاموش‌کردن پروکسی سیستم", true, None::<&str>)?;
    #[cfg(windows)]
    let rebuild = MenuItem::with_id(app, "widget_rebuild", "بازسازی ویجت تسک‌بار", true, None::<&str>)?;
    let quiet = CheckMenuItem::with_id(app, "quiet", "حالت سکوت", true, false, None::<&str>)?;
    let mut items = Vec::new();
    for (id, label) in LAYOUTS { if (*id == "title" && !cfg!(target_os = "macos")) || (*id == "widget" && !cfg!(windows)) { continue; } items.push((*id, CheckMenuItem::with_id(app, format!("layout:{id}"), *label, true, layout(app) == *id, None::<&str>)?)); }
    let state = app.state::<AppState>();
    let settings = state.settings.read();
    let d0 = settings.tray_down;
    let u0 = settings.tray_up;
    let sep = PredefinedMenuItem::separator(app)?;
    let t_down = CheckMenuItem::with_id(app, "toggle:down", "آیکون دانلود", true, d0, None::<&str>)?;
    let t_up = CheckMenuItem::with_id(app, "toggle:up", "آیکون آپلود", true, u0, None::<&str>)?;
    let mut refs: Vec<&dyn tauri::menu::IsMenuItem<_>> = items.iter().map(|(_, item)| item as &dyn tauri::menu::IsMenuItem<_>).collect();
    refs.push(&sep); refs.push(&t_down); refs.push(&t_up);
    let layout_menu = Submenu::with_items(app, "نمایش روی تسک‌بار", true, &refs)?;
    *LAYOUT_ITEMS.lock() = Some(items);
    *TOGGLE_ITEMS.lock() = Some((t_down, t_up));
    let quit = MenuItem::with_id(app, "quit", "خروج", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    #[cfg(windows)]
    let menu = Menu::with_items(app, &[&open, &check, &proxy_item, &quiet, &layout_menu, &rebuild, &separator, &quit])?;
    #[cfg(not(windows))]
    let menu = Menu::with_items(app, &[&open, &check, &proxy_item, &quiet, &layout_menu, &separator, &quit])?;
    *MENU.lock() = Some(menu.clone());
    TrayIconBuilder::with_id(TRAY_ID).icon(icon::make([120,120,120,255], icon::Shape::Solid, None)).tooltip("NetWatch").menu(&menu).show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            if let Some(value) = event.id().as_ref().strip_prefix("layout:") { let state = app.state::<AppState>(); { let mut settings = state.settings.write(); settings.tray_layout = value.into(); let _ = config::save(&state.settings_path.lock(), &settings); } refresh_icon(app); return; }
            if let Some(which) = event.id().as_ref().strip_prefix("toggle:") { let st = app.state::<AppState>(); { let mut settings = st.settings.write(); if which == "down" { settings.tray_down = !settings.tray_down; } else { settings.tray_up = !settings.tray_up; } if settings.tray_layout != "separate" && (settings.tray_down || settings.tray_up) { settings.tray_layout = "separate".into(); } let _ = config::save(&st.settings_path.lock(), &settings); } refresh_icon(app); return; }
            match event.id().as_ref() { "open" => popup::open_dashboard(app), "check" => app.state::<AppState>().kick.notify_one(), "proxy" => { let _ = proxy::disable(); app.state::<AppState>().kick.notify_one(); }, "quiet" => { let state = app.state::<AppState>(); let mut settings = state.settings.write(); settings.quiet = !settings.quiet; let _ = config::save(&state.settings_path.lock(), &settings); }, "widget_rebuild" => { #[cfg(windows)] crate::taskbar::rebuild(app); }, "quit" => app.exit(0), _ => {} }
        })
        .on_tray_icon_event(|tray, event| { let app = tray.app_handle(); match event { TrayIconEvent::Enter { .. } => { let state = app.state::<AppState>(); if state.last_check.lock().elapsed() > Duration::from_secs(3) && !state.checking.load(SeqCst) { state.kick.notify_one(); } popup::close(&app); crate::hover::schedule_show(&app); }, TrayIconEvent::Leave { .. } => crate::hover::schedule_hide(&app), TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } => { popup::close(&app); crate::hover::schedule_hide(&app); popup::open_dashboard(&app); }, _ => {} } })
        .build(app)?;
    Ok(())
}

pub fn ensure_speed_tray(app: &AppHandle) -> tauri::Result<()> {
    let state = app.state::<AppState>();
    let settings = state.settings.read();
    let lay = settings.tray_layout.clone();
    let down = settings.tray_down;
    let up = settings.tray_up;
    #[cfg(windows)]
    crate::taskbar::ensure(app, lay == "widget");
    let k = dpi_scale(app);
    sync_tray(app, SPEED_TRAY_ID, lay == "dual", || icon::speed_text("0.0", "0.0", k), "NetWatch — سرعت")?;
    sync_tray(app, DOWN_TRAY_ID, lay == "separate" && down, || icon::speed_single("0.0", true, k), "NetWatch — دانلود")?;
    sync_tray(app, UP_TRAY_ID, lay == "separate" && up, || icon::speed_single("0.0", false, k), "NetWatch — آپلود")?;
    if layout(app) != "title" { if let Some(tray) = app.tray_by_id(TRAY_ID) { let _ = tray.set_title(None::<&str>); } }
    *LAST_SPEED.lock() = None;
    *LAST_DOWN.lock() = None;
    *LAST_UP.lock() = None;
    Ok(())
}

pub fn on_speed(app: &AppHandle, rx_bps: u64, tx_bps: u64) {
    let state = { app.state::<AppState>().snapshot.lock().state };
    let key = (icon::format_mb(rx_bps), icon::format_mb(tx_bps), state);
    { let mut l = LAST_SPEED.lock(); if l.as_ref() == Some(&key) { return; } *l = Some(key.clone()); }
    let (down, up) = (key.0.clone(), key.1.clone());
    let lay = layout(app);
    let scale = dpi_scale(app);
    let img = match lay.as_str() { "dual" => Some(icon::speed_text(&down, &up, scale)), "badge" => Some(icon::badge(&down, &up, state.color(), scale)), _ => None };
    on_main(app, move |a| match lay.as_str() {
        "dual" => if let Some(t) = a.tray_by_id(SPEED_TRAY_ID) { let _ = t.set_icon(img); let _ = t.set_tooltip(Some(format!("{RLM}دانلود: {}  ·  آپلود: {}", ltr(format!("{down} MB/s")), ltr(format!("{up} MB/s"))))); },
        "badge" => if let Some(t) = a.tray_by_id(TRAY_ID) { let _ = t.set_icon(img); *LAST_ICON.lock() = None; },
        "title" => if let Some(t) = a.tray_by_id(TRAY_ID) { let _ = t.set_title(Some(format!("↓{down} ↑{up}"))); },
        "separate" => {
            let do_down = { let mut l = LAST_DOWN.lock(); if l.as_deref() == Some(&down) { false } else { *l = Some(down.clone()); true } };
            let do_up = { let mut l = LAST_UP.lock(); if l.as_deref() == Some(&up) { false } else { *l = Some(up.clone()); true } };
            if do_down { if let Some(t) = a.tray_by_id(DOWN_TRAY_ID) { let _ = t.set_icon(Some(icon::speed_single(&down, true, scale))); let _ = t.set_tooltip(Some(format!("{RLM}دانلود: {}", ltr(format!("{down} MB/s"))))); } }
            if do_up { if let Some(t) = a.tray_by_id(UP_TRAY_ID) { let _ = t.set_icon(Some(icon::speed_single(&up, false, scale))); let _ = t.set_tooltip(Some(format!("{RLM}آپلود: {}", ltr(format!("{up} MB/s"))))); } }
        }
        _ => {}
    });
}

pub fn set_state_icon(app: &AppHandle, state: NetState) {
    if layout(app) == "badge" { let state_ref = app.state::<AppState>(); let (rx, tx) = { let speed = state_ref.speed.lock(); (speed.rx_bps, speed.tx_bps) }; *LAST_SPEED.lock() = None; on_speed(app, rx, tx); return; }
    if *LAST_ICON.lock() == Some(state) { return; }
    let img = icon::make(state.color(), shape(state), None);
    on_main(app, move |a| { if let Some(t) = a.tray_by_id(TRAY_ID) { if t.set_icon(Some(img)).is_ok() { *LAST_ICON.lock() = Some(state); } } });
}

pub fn refresh_icon(app: &AppHandle) { let _ = ensure_speed_tray(app); *LAST_ICON.lock() = None; *LAST_SPEED.lock() = None; *LAST_DOWN.lock() = None; *LAST_UP.lock() = None; let state = app.state::<AppState>().snapshot.lock().state; set_state_icon(app, state); sync_layout_menu(app); }
fn sync_layout_menu(app: &AppHandle) {
    let current = layout(app);
    if let Some(items) = LAYOUT_ITEMS.lock().as_ref() {
        for (id, item) in items {
            let _ = item.set_checked(*id == current);
        }
    }
    let state = app.state::<AppState>();
    let settings = state.settings.read();
    let down = settings.tray_down;
    let up = settings.tray_up;
    let separate = settings.tray_layout == "separate";
    if let Some((td, tu)) = TOGGLE_ITEMS.lock().as_ref() {
        let _ = td.set_checked(down);
        let _ = tu.set_checked(up);
        let _ = td.set_enabled(separate);
        let _ = tu.set_enabled(separate);
    }
}

pub fn spin(app: &AppHandle) -> Option<JoinHandle<()>> {
    if cfg!(target_os = "linux") || layout(app) == "badge" { return None; }
    let app = app.clone();
    Some(tauri::async_runtime::spawn(async move {
        let st = app.state::<AppState>();
        let mut i = 0u32;
        tokio::time::sleep(Duration::from_millis(400)).await;
        while st.checking.load(SeqCst) {
            let state_val = { st.snapshot.lock().state };
            let img = icon::make(state_val.color(), shape(state_val), Some(i as f32 * 0.785));
            on_main(&app, move |a| { if let Some(t) = a.tray_by_id(TRAY_ID) { let _ = t.set_icon(Some(img)); } });
            *LAST_ICON.lock() = None;
            i = (i + 1) % 8;
            tokio::time::sleep(Duration::from_millis(120)).await;
        }
    }))
}

fn tooltip_text(app: &AppHandle, snapshot: &Snapshot) -> String { let state = app.state::<AppState>(); let format_ms = |value: Option<u32>| value.map(|ms| ltr(format!("{ms} ms"))).unwrap_or_else(|| "—".into()); let (rx, tx) = { let speed = state.speed.lock(); (crate::speed::fmt(speed.rx_bps, true), crate::speed::fmt(speed.tx_bps, true)) }; let mut lines = vec![format!("{RLM}{} {}", snapshot.state.emoji(), snapshot.state.title()), format!("داخلی: {}  ·  خارجی: {}", format_ms(snapshot.domestic_ping), format_ms(snapshot.international_ping)), format!("دانلود: {}  ·  آپلود: {}", ltr(rx), ltr(tx))]; if let Some(note) = &snapshot.note { lines.push(format!("{RLM}{note}")); } lines.join("\n") }
pub fn update_tooltip(app: &AppHandle, snapshot: &Snapshot) { let text = if cfg!(target_os = "linux") { tooltip_text(app, snapshot) } else { format!("{RLM}NetWatch — {}", snapshot.state.title()) }; on_main(app, move |a| { if let Some(t) = a.tray_by_id(TRAY_ID) { let _ = t.set_tooltip(Some(text)); } crate::hover::refresh(a); }); }
pub fn update(app: &AppHandle, snapshot: &Snapshot) { set_state_icon(app, snapshot.state); update_tooltip(app, snapshot); }
