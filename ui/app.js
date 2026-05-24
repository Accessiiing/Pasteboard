// ---------- Tauri 桥接（无 Tauri 环境时回退到浏览器预览 mock）----------
const TAURI = window.__TAURI__;
const hasTauri = !!TAURI;

async function invoke(cmd, args) {
  if (hasTauri) return TAURI.core.invoke(cmd, args);
  return mockInvoke(cmd, args);
}
async function listen(event, cb) {
  if (hasTauri) return TAURI.event.listen(event, cb);
  return () => {};
}

// ---------- 浏览器预览用的假数据 ----------
const MOCK = [
  { id: 1, kind: "text", preview: "API_KEY=sk-ant-api03-xxxxxxxxxxxxxx", thumb: null, pinned: true, created_at: Date.now() - 5000, last_used_at: Date.now() },
  { id: 2, kind: "text", preview: "在 Figma 中，AutoLayout 的 fill_container 属性让子元素可以自动填充父容器的剩余空间，这对响应式设计非常关键。", thumb: null, pinned: false, created_at: Date.now() - 120000, last_used_at: Date.now() - 120000 },
  { id: 3, kind: "image", preview: "[图片]", thumb: "data:image/svg+xml;base64," + btoa('<svg xmlns="http://www.w3.org/2000/svg" width="320" height="120"><rect width="320" height="120" fill="#222"/><rect x="120" y="30" width="80" height="55" fill="#999"/></svg>'), pinned: false, created_at: Date.now() - 300000, last_used_at: Date.now() - 300000 },
  { id: 4, kind: "link", preview: "https://github.com/anthropics/anthropic-sdk-python", thumb: null, pinned: false, created_at: Date.now() - 720000, last_used_at: Date.now() - 720000 },
  { id: 5, kind: "text", preview: "会议纪要：本次产品评审会议确定 Q4 版本需优先完成用户反馈模块与数据看板重构，预期上线时间 11 月底。", thumb: null, pinned: false, created_at: Date.now() - 3600000, last_used_at: Date.now() - 3600000 },
];
let mockSettings = { save_path: "C:/Users/User/Documents/ClipNest", max_records: 200, clear_on_shutdown: false, autostart: true, hotkey: "Alt+V" };
function mockInvoke(cmd, args) {
  switch (cmd) {
    case "list_records": return Promise.resolve(MOCK);
    case "get_settings": return Promise.resolve(mockSettings);
    case "delete_record": { const i = MOCK.findIndex(r => r.id === args.id); if (i >= 0) MOCK.splice(i, 1); return Promise.resolve(); }
    case "pin_record": { const r = MOCK.find(x => x.id === args.id); if (r) r.pinned = args.pinned; return Promise.resolve(); }
    case "choose_folder": return Promise.resolve("D:/Saved/Pasteboard");
    default: return Promise.resolve();
  }
}

// ---------- 工具 ----------
function escapeHtml(s) {
  return s.replace(/[&<>"']/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
}
function relativeTime(ms) {
  const diff = Date.now() - ms;
  const s = Math.floor(diff / 1000);
  if (s < 60) return "刚刚";
  const m = Math.floor(s / 60);
  if (m < 60) return `${m} 分钟前`;
  const h = Math.floor(m / 60);
  if (h < 24) return `${h} 小时前`;
  return `${Math.floor(h / 24)} 天前`;
}
const TYPE_LABEL = { text: "纯文本", link: "链接", image: "截图" };

const ICON = {
  trash: '<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"/><path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6"/><path d="M10 11v6M14 11v6"/></svg>',
  pin: '<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 17v5"/><path d="M9 2h6l-1 7 3 3v2H7v-2l3-3z"/></svg>',
  save: '<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>',
};

// ---------- 列表渲染 ----------
const listEl = document.getElementById("list");
const footerEl = document.getElementById("footer");
let allRecords = [];
let searchTerm = "";

async function loadRecords() {
  allRecords = await invoke("list_records");
  renderList();
}

function renderList() {
  const term = searchTerm.trim().toLowerCase();
  const records = term
    ? allRecords.filter(r => (r.preview || "").toLowerCase().includes(term))
    : allRecords;

  listEl.innerHTML = "";
  if (records.length === 0) {
    const e = document.createElement("div");
    e.className = "empty";
    e.textContent = term ? "没有匹配的记录" : "暂无剪贴板记录";
    listEl.appendChild(e);
  } else {
    for (const r of records) listEl.appendChild(buildCard(r));
  }
  footerEl.textContent = `点击条目即可粘贴 · 共 ${allRecords.length} 条记录`;
}

function buildCard(r) {
  const card = document.createElement("div");
  card.className = "card";
  card.dataset.id = r.id;

  let bodyHtml = "";
  if (r.pinned) {
    bodyHtml += `<div class="pin-badge">${ICON.pin}已固定</div>`;
  }
  if (r.kind === "image") {
    bodyHtml += `<div class="card-image">${r.thumb ? `<img src="${r.thumb}" alt="图片"/>` : "图片"}</div>`;
  } else {
    const cls = r.kind === "link" ? "card-text card-link" : "card-text";
    bodyHtml += `<div class="${cls}">${escapeHtml(r.preview || "")}</div>`;
  }
  bodyHtml += `<div class="card-meta">${relativeTime(r.created_at)} · ${TYPE_LABEL[r.kind] || "纯文本"}</div>`;

  bodyHtml += `<button class="dots" data-action="menu" title="更多">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><circle cx="5" cy="12" r="1.8"/><circle cx="12" cy="12" r="1.8"/><circle cx="19" cy="12" r="1.8"/></svg>
    </button>
    <div class="dropdown">
      <button class="menu-item danger" data-action="delete">${ICON.trash}删除</button>
      <button class="menu-item" data-action="pin">${ICON.pin}${r.pinned ? "取消固定" : "固定"}</button>
      <button class="menu-item" data-action="save">${ICON.save}保存</button>
    </div>`;

  card.innerHTML = bodyHtml;
  return card;
}

// 事件委托
listEl.addEventListener("click", async (e) => {
  const card = e.target.closest(".card");
  if (!card) return;
  const id = Number(card.dataset.id);
  const actionEl = e.target.closest("[data-action]");
  const action = actionEl ? actionEl.dataset.action : null;

  if (action === "menu") {
    e.stopPropagation();
    const dd = card.querySelector(".dropdown");
    const isShown = dd.classList.contains("show");
    closeAllMenus();
    if (!isShown) dd.classList.add("show");
    return;
  }
  if (action === "delete") { e.stopPropagation(); await invoke("delete_record", { id }); await loadRecords(); return; }
  if (action === "pin") {
    e.stopPropagation();
    const rec = allRecords.find(r => r.id === id);
    await invoke("pin_record", { id, pinned: !(rec && rec.pinned) });
    await loadRecords();
    return;
  }
  if (action === "save") {
    e.stopPropagation();
    closeAllMenus();
    try { const dest = await invoke("save_record", { id }); showToast(dest ? "已保存到 " + dest : "已保存"); }
    catch (err) { showToast("保存失败：" + err); }
    return;
  }

  // 点击卡片主体 -> 粘贴
  card.classList.add("copied");
  await invoke("paste_record", { id });
});

function closeAllMenus() {
  document.querySelectorAll(".dropdown.show").forEach(d => d.classList.remove("show"));
}
document.addEventListener("click", (e) => {
  if (!e.target.closest(".dots") && !e.target.closest(".dropdown")) closeAllMenus();
});

// ---------- 顶部操作 ----------
document.getElementById("btn-search").addEventListener("click", () => {
  const bar = document.getElementById("search-bar");
  bar.classList.toggle("show");
  if (bar.classList.contains("show")) document.getElementById("search-input").focus();
  else { searchTerm = ""; document.getElementById("search-input").value = ""; renderList(); }
});
document.getElementById("search-input").addEventListener("input", (e) => {
  searchTerm = e.target.value; renderList();
});
document.getElementById("btn-clear").addEventListener("click", async () => {
  if (allRecords.length === 0) return;
  if (confirm("确定要清空全部剪贴板记录吗？（固定项不受影响）")) {
    await invoke("clear_all"); await loadRecords();
  }
});

// ---------- 视图切换 ----------
function showView(name) {
  document.getElementById("view-clipboard").classList.toggle("active", name === "clipboard");
  document.getElementById("view-settings").classList.toggle("active", name === "settings");
  closeAllMenus();
  if (name === "settings") loadSettings();
}
document.getElementById("btn-back").addEventListener("click", () => showView("clipboard"));

// ---------- 设置 ----------
const savePathEl = document.getElementById("save-path");
const maxRecordsEl = document.getElementById("max-records");
const clearShutdownEl = document.getElementById("clear-shutdown");
const autostartEl = document.getElementById("autostart");
const hotkeyCapsEl = document.getElementById("hotkey-caps");
let currentSettings = null;

function renderHotkeyCaps(spec) {
  const parts = spec.split("+");
  hotkeyCapsEl.innerHTML = parts
    .map((p, i) => `${i ? '<span class="keycap-plus">+</span>' : ""}<span class="keycap">${escapeHtml(p)}</span>`)
    .join("");
}

async function loadSettings() {
  currentSettings = await invoke("get_settings");
  savePathEl.value = currentSettings.save_path;
  maxRecordsEl.value = currentSettings.max_records;
  clearShutdownEl.checked = currentSettings.clear_on_shutdown;
  autostartEl.checked = currentSettings.autostart;
  renderHotkeyCaps(currentSettings.hotkey);
}

async function pushSettings() {
  let max = parseInt(maxRecordsEl.value, 10);
  if (isNaN(max)) max = 200;
  max = Math.min(300, Math.max(100, max));
  maxRecordsEl.value = max;
  await invoke("set_settings", {
    savePath: savePathEl.value,
    maxRecords: max,
    clearOnShutdown: clearShutdownEl.checked,
    autostart: autostartEl.checked,
  });
}

document.getElementById("btn-folder").addEventListener("click", async () => {
  const picked = await invoke("choose_folder");
  if (picked) { savePathEl.value = picked; await pushSettings(); showToast("保存位置已更新"); }
});
maxRecordsEl.addEventListener("change", pushSettings);
clearShutdownEl.addEventListener("change", pushSettings);
autostartEl.addEventListener("change", pushSettings);

// 快捷键录入
const editBtn = document.getElementById("btn-edit-hotkey");
const hotkeySub = document.getElementById("hotkey-sub");
let recording = false;
editBtn.addEventListener("click", () => {
  recording = !recording;
  editBtn.classList.toggle("recording", recording);
  hotkeySub.textContent = recording ? "请按下组合键…" : "点击右侧按键组合录入快捷键";
});
window.addEventListener("keydown", async (e) => {
  if (!recording) return;
  e.preventDefault();
  const key = e.key;
  if (["Control", "Shift", "Alt", "Meta"].includes(key)) return; // 等待主键
  const mods = [];
  if (e.ctrlKey) mods.push("Ctrl");
  if (e.altKey) mods.push("Alt");
  if (e.shiftKey) mods.push("Shift");
  if (e.metaKey) mods.push("Win");
  if (mods.length === 0) return; // 必须带修饰键
  let main = key.length === 1 ? key.toUpperCase() : key;
  const spec = [...mods, main].join("+");
  recording = false;
  editBtn.classList.remove("recording");
  hotkeySub.textContent = "点击右侧按键组合录入快捷键";
  renderHotkeyCaps(spec);
  try { await invoke("set_hotkey", { spec }); showToast("快捷键已更新为 " + spec); }
  catch (err) { showToast("快捷键设置失败：" + err); }
});

// ---------- Toast ----------
let toastTimer = null;
function showToast(msg) {
  const t = document.getElementById("toast");
  t.textContent = msg;
  t.classList.add("show");
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => t.classList.remove("show"), 1800);
}

// ---------- Esc 隐藏窗口 ----------
window.addEventListener("keydown", (e) => {
  if (e.key === "Escape" && hasTauri) invoke("hide_window");
});

// ---------- 后端事件 ----------
listen("records-updated", () => loadRecords());
listen("navigate", (e) => showView(e.payload));

// ---------- 初始化 ----------
loadRecords();
