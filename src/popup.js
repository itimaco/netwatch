const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const $ = (id) => document.getElementById(id);
const call = (name, args) => invoke(name, args).catch(() => {});
let cur = null;

function render(p) {
  cur = p;
  document.documentElement.style.setProperty("--c", p.color);
  document.documentElement.style.setProperty("--secs", `${p.seconds || 4}s`);
  $("title").textContent = p.title;
  $("hint").textContent = p.ping != null ? `${p.hint} · ${p.ping}ms` : p.hint;
  $("t").title = `${p.title} — ${p.hint}`;
  const act = $("act");
  if (p.action) { act.textContent = p.action[1]; act.classList.remove("hidden"); } else act.classList.add("hidden");
  // پنجره بسته نمی‌شود بلکه پنهان می‌شود، پس انیمیشن‌ها را دستی از نو اجرا می‌کنیم.
  for (const el of [$("t"), $("bar")]) { el.style.animation = "none"; void el.offsetHeight; el.style.animation = ""; }
}

// نگه‌داشتن پاپ‌آپ فقط تا وقتی ضربان می‌فرستیم معتبر است. هر ضربان دوباره
// چک می‌کند که موس واقعاً روی پاپ‌آپ است، تا گم‌شدن یک mouseleave
// (که روی پنجره‌ی بدون فوکوس ویندوز پیش می‌آید) پاپ‌آپ را گیر نیندازد.
let beat = 0;
const over = () => !!document.querySelector("#t:hover");
function release() { if (beat) { clearInterval(beat); beat = 0; } call("popup_hold", { hold: false }); }
function grab() {
  if (beat) return;
  call("popup_hold", { hold: true });
  beat = setInterval(() => { if (over() && document.visibilityState === "visible") call("popup_hold", { hold: true }); else release(); }, 500);
}

call("popup_ready").then((p) => p && render(p));
listen("popup-data", (e) => { release(); render(e.payload); if (over()) grab(); });
$("t").addEventListener("mouseenter", grab);
$("t").addEventListener("mouseleave", release);
document.addEventListener("mouseleave", release);
document.addEventListener("visibilitychange", () => { if (document.visibilityState !== "visible") release(); });
window.addEventListener("blur", release);
$("t").addEventListener("click", (e) => { if (e.target.closest("button")) return; release(); call("open_dashboard"); call("popup_close"); });
$("x").onclick = (e) => { e.stopPropagation(); release(); call("popup_close"); };
$("act").onclick = async (e) => {
  e.stopPropagation();
  if (!cur?.action) return;
  release();
  if (cur.action[0] === "disable_proxy") await call("disable_proxy"); else await call("open_dashboard");
  call("popup_close");
};
