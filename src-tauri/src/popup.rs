use crate::{state::Snapshot, AppState};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

const LABEL: &str = "popup";
const W: f64 = 300.0;
const H: f64 = 56.0;
/// هر «نگه‌داشتن» فقط یک اجاره‌ی کوتاه است؛ اگر ضربان از سمت صفحه نرسد خودش منقضی می‌شود.
const HOLD_LEASE: Duration = Duration::from_millis(1500);
/// سقف مطلق نگه‌داشتن با موس، تا پاپ‌آپ هیچ‌وقت برای همیشه نماند.
const MAX_HOLD: Duration = Duration::from_secs(20);
const TICK: Duration = Duration::from_millis(150);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopupPayload { pub state:String, pub color:String, pub title:String, pub hint:String, pub action:Option<(String,String)>, pub ping:Option<u32>, pub seconds:u64 }

/// تنها منبع حقیقتِ عمر پاپ‌آپ. هیچ تسک جداگانه‌ای مالک بستن نیست.
#[derive(Debug, Default, Clone, Copy)]
pub struct Timer { pub hide_at: Option<Instant>, pub hold_until: Option<Instant>, pub hard_at: Option<Instant> }

pub fn position(app: &AppHandle) -> (f64, f64) {
    let st = app.state::<AppState>();
    let anchor = st.settings.read().anchor_tray;
    let (mw, mh) = app.primary_monitor().ok().flatten().map(|m| { let s = m.size().to_logical::<f64>(m.scale_factor()); (s.width, s.height) }).unwrap_or((1280.0, 720.0));
    if anchor {
        if let Some((x, y, w, h)) = *st.tray_rect.lock() {
            let cx = (x + w / 2.0 - W / 2.0).clamp(8.0, mw - W - 8.0);
            let cy = if cfg!(target_os = "macos") { y + h + 6.0 } else { y - H - 8.0 };
            return (cx, cy.clamp(8.0, mh - H - 8.0));
        }
    }
    if cfg!(target_os = "macos") { (mw - W - 12.0, 36.0) } else { (mw - W - 12.0, mh - H - 56.0) }
}

pub fn show(app: &AppHandle, s: &Snapshot, seconds: u64) {
    let seconds = seconds.clamp(2, 30);
    let payload = PopupPayload {
        state: s.state.name(), color: s.state.hex(), title: s.state.title().into(),
        hint: s.note.clone().unwrap_or_else(|| s.state.hint().into()),
        action: s.state.action().map(|(a, b)| (a.into(), b.into())),
        ping: if s.state == crate::state::NetState::Online { s.international_ping } else { None },
        seconds,
    };
    let st = app.state::<AppState>();
    *st.pending_popup.lock() = Some(payload.clone());
    let now = Instant::now();
    let life = Duration::from_secs(seconds);
    *st.popup.lock() = Timer { hide_at: Some(now + life), hold_until: None, hard_at: Some(now + life + MAX_HOLD) };

    let (x, y) = position(app);
    // پنجره یک‌بار ساخته می‌شود و بعد از آن فقط پنهان/آشکار می‌شود؛
    // ساخت و بستن پیاپی روی ویندوز باعث مسابقه و جا ماندن پاپ‌آپ می‌شد.
    let (win, fresh) = match app.get_webview_window(LABEL) {
        Some(w) => (w, false),
        None => match WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("popup.html".into()))
            .title("NetWatch").decorations(false).transparent(true).always_on_top(true).skip_taskbar(true)
            .resizable(false).focused(false).shadow(false).visible(false).inner_size(W, H).position(x, y).build()
        { Ok(w) => (w, true), Err(_) => { *st.popup.lock() = Timer::default(); return } },
    };
    tauri::async_runtime::spawn(async move {
        if fresh { tokio::time::sleep(Duration::from_millis(150)).await; }
        let _ = win.set_position(tauri::LogicalPosition::new(x, y));
        let _ = win.emit("popup-data", &payload);
        let _ = win.show();
    });
}

/// درخواست نگه‌داشتن از سمت صفحه: یک اجاره‌ی کوتاه که باید مدام تمدید شود.
pub fn hold(app: &AppHandle, on: bool) {
    let st = app.state::<AppState>();
    let mut timer = st.popup.lock();
    if !on { timer.hold_until = None; return; }
    if timer.hide_at.is_none() { return; }
    timer.hold_until = Some(Instant::now() + HOLD_LEASE);
}

pub fn close(app: &AppHandle) {
    let st = app.state::<AppState>();
    *st.popup.lock() = Timer::default();
    if let Some(w) = app.get_webview_window(LABEL) { let _ = w.hide(); }
}

/// تنها جایی که پاپ‌آپ بسته می‌شود؛ یک‌بار در setup اجرا می‌شود.
pub async fn watchdog(app: AppHandle) {
    loop {
        tokio::time::sleep(TICK).await;
        let now = Instant::now();
        let timer = *app.state::<AppState>().popup.lock();
        let done = match timer.hide_at {
            Some(hide_at) => {
                let held = timer.hold_until.map_or(false, |until| until > now);
                let hard = timer.hard_at.map_or(true, |at| now >= at);
                hard || (now >= hide_at && !held)
            }
            // تورِ ایمنی: پنجره‌ی باز بدون تایمر (هر باگ ناشناخته) هم جمع می‌شود.
            None => app.get_webview_window(LABEL).and_then(|w| w.is_visible().ok()).unwrap_or(false),
        };
        if done { close(&app); }
    }
}

pub fn open_dashboard(app: &AppHandle) { if let Some(w) = app.get_webview_window("main") { let _ = w.show(); let _ = w.unminimize(); let _ = w.set_focus(); } }
