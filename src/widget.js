const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const $ = (id) => document.getElementById(id);
const mb = (bps) => { const value = bps / 1048576; return value < 0.05 ? "0.0" : value < 10 ? value.toFixed(1) : value < 1000 ? value.toFixed(0) : "999"; };
function speed(value) { $("rx").textContent = mb(value.rx_bps); $("tx").textContent = mb(value.tx_bps); }
invoke("get_speed").then(speed); listen("speed", (event) => speed(event.payload));
invoke("get_settings").then((settings) => $("w").classList.toggle("pill", !!settings.widget_pill));
listen("widget-size", (event) => $("w").classList.toggle("compact", event.payload.h < 34));
const widget = $("w");
const ping = () => invoke("widget_alive", { visible: document.visibilityState === "visible" }).catch(() => {});
setInterval(ping, 1000);
document.addEventListener("visibilitychange", ping);
ping();
widget.onmouseenter = () => invoke("widget_hover", { on: true });
widget.onmouseleave = () => invoke("widget_hover", { on: false });
widget.onclick = () => { invoke("widget_hover", { on: false }); invoke("open_dashboard"); };
widget.oncontextmenu = (event) => { event.preventDefault(); invoke("widget_hover", { on: false }); invoke("widget_menu"); };
