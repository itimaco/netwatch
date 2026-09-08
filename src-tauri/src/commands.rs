use crate::{config::{self,Settings},history::{self,EventRow,Stats},popup::PopupPayload,probes::proxy,state::Snapshot,popup,AppState};use std::sync::atomic::Ordering::SeqCst;use tauri::{AppHandle,Manager,State};
#[tauri::command]pub fn get_snapshot(st:State<AppState>)->Snapshot{st.snapshot.lock().clone()}
#[tauri::command]pub fn get_speed(st:State<AppState>)->crate::speed::Speed{st.speed.lock().clone()}
#[tauri::command]pub async fn speed_test(app:AppHandle)->Result<crate::speed::SpeedTest,String>{let state=app.state::<AppState>();let(down,up)={let settings=state.settings.read();(settings.speedtest_down.clone(),settings.speedtest_up.clone())};crate::speed::test(app,&down,&up).await.map_err(|error|error.to_string())}
#[tauri::command]pub fn run_now(st:State<AppState>){st.kick.notify_one()}
#[tauri::command]pub fn get_settings(st:State<AppState>)->Settings{st.settings.read().clone()}
#[tauri::command]pub fn set_settings(app:AppHandle,st:State<AppState>,settings:Settings)->Result<(),String>{config::save(&st.settings_path.lock(),&settings).map_err(|e|e.to_string())?;*st.settings.write()=settings;crate::tray::refresh_icon(&app);st.kick.notify_one();Ok(())}
#[tauri::command]pub fn get_events(st:State<AppState>,limit:Option<u32>)->Vec<EventRow>{history::events(&st,limit.unwrap_or(100))}
#[tauri::command]pub fn get_stats(st:State<AppState>,days:Option<u32>)->Stats{history::stats(&st,days.unwrap_or(1))}
#[tauri::command]pub fn get_analytics(st:State<AppState>,period:Option<String>)->history::Analytics{history::analytics(&st,period.as_deref().unwrap_or("day"))}
#[tauri::command]pub fn disable_proxy(st:State<AppState>)->Result<(),String>{proxy::disable().map_err(|e|e.to_string())?;st.kick.notify_one();Ok(())}
#[tauri::command]pub fn popup_hold(st:State<AppState>,hold:bool){st.popup_hold.store(hold,SeqCst)}
#[tauri::command]pub fn popup_close(app:AppHandle){popup::close(&app);}
#[tauri::command]pub fn popup_ready(st:State<AppState>)->Option<PopupPayload>{st.pending_popup.lock().clone()}
#[tauri::command]pub fn open_dashboard(app:AppHandle){popup::open_dashboard(&app)}
#[tauri::command]pub fn widget_menu(app:AppHandle){#[cfg(windows)]crate::taskbar::popup_menu(&app)}
#[tauri::command]pub fn widget_hover(app:AppHandle,on:bool){#[cfg(windows)]crate::taskbar::hover(&app,on)}
#[tauri::command]pub fn widget_alive(visible:bool){#[cfg(windows)]crate::taskbar::alive(visible);#[cfg(not(windows))]let _=visible;}
#[tauri::command]pub fn widget_rebuild(app:AppHandle){#[cfg(windows)]crate::taskbar::rebuild(&app);#[cfg(not(windows))]let _=app;}
