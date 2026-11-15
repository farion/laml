// Browser platform adapter only. Document management UI stays in shared Rust/egui.
let db, records = new Map(), versions = new Map(), pending = new Map();
let chain = Promise.resolve(), saving = false, storageError = '', notice = '';
let session = '', connection = 'Connecting to server…', admitted = false;
let actions = [];
let imports = [], restoredTabs = [], openTabs = [], channel;
const randomId = () => Array.from(crypto.getRandomValues(new Uint8Array(16)), b => b.toString(16).padStart(2, '0')).join('');
const request = req => new Promise((resolve, reject) => {
  req.onsuccess = () => resolve(req.result); req.onerror = () => reject(req.error);
});
const complete = tx => {
  const done = new Promise((resolve, reject) => {
    tx.oncomplete = resolve; tx.onerror = tx.onabort = () => reject(tx.error || new Error('Browser storage transaction failed'));
  });
  // A failed request may reject before the caller reaches `await done`.
  // Keep that transaction rejection handled; awaiting it still reports failure.
  done.catch(() => {});
  return done;
};
const errorText = e => `Could not save in this browser: ${e?.message || e}. Download your LAML before closing this page.`;
async function refresh() {
  if (!db) return;
  const tx = db.transaction('documents', 'readonly');
  const done = complete(tx);
  const rows = await request(tx.objectStore('documents').getAll()); await done;
  records = new Map(rows.map(r => [r.id, r]));
  for (const [id, row] of pending) records.set(id, row);
}
export async function initialize() {
  try {
    const open = indexedDB.open('laml-browser-library', 1);
    open.onupgradeneeded = () => {
      open.result.createObjectStore('documents', {keyPath: 'id'});
      open.result.createObjectStore('meta');
    };
    db = await request(open);
    db.onversionchange = () => { db.close(); storageError = 'Browser storage was changed by another tab. Download unsaved LAML, then reload.'; };
    const tx = db.transaction('meta', 'readwrite'), done = complete(tx), meta = tx.objectStore('meta');
    session = await request(meta.get('session'));
    if (!session) { session = randomId(); meta.put(session, 'session'); }
    await done; await refresh();
    try { restoredTabs = JSON.parse(sessionStorage.getItem('laml.open-documents') || '[]'); } catch (_) {}
    channel = typeof BroadcastChannel === 'function' ? new BroadcastChannel('laml-browser-library') : null;
    if (channel) channel.onmessage = () => { chain = chain.then(refresh).catch(e => { storageError = errorText(e); }); };
  } catch (e) { storageError = errorText(e); session = randomId(); }
  await heartbeat();
  setInterval(heartbeat, 20000);
  setInterval(() => { if (pending.size && !saving) flush(); }, 2000);
  window.addEventListener('online', heartbeat);
  document.addEventListener('visibilitychange', () => { if (!document.hidden) heartbeat(); });
  window.addEventListener('beforeunload', e => {
    if (pending.size || storageError) { e.preventDefault(); e.returnValue = 'Changes are not saved in this browser yet'; }
  });
}
async function heartbeat() {
  try {
    const r = await fetch('/api/session', {method: 'POST', headers: {'X-LAML-Session': session}});
    admitted = r.ok;
    connection = r.ok ? '' : r.status === 503
      ? 'The server is full. All available places are occupied. Local editing remains available; retrying automatically.'
      : 'Server unavailable. Local editing and browser storage remain available; retrying automatically.';
  } catch (_) { admitted = false; connection = 'Offline. Local editing and browser storage remain available; retrying automatically.'; }
}
export function sessionId() { return session; }
export function available() { return admitted; }
export function connectionStatus() { return connection; }
export function storageStatus() { return storageError || (pending.size ? 'Saving in this browser…' : 'Saved in this browser'); }
export function storageFailed() { return !!storageError; }
export function safeToClose() { return pending.size === 0 && !storageError; }
export function newId() { return randomId(); }
export function snapshot() {
  return JSON.stringify({records: [...records.values()].sort((a,b) => b.updated-a.updated), notice, error: storageError});
}
export function startupTabs() { return JSON.stringify(restoredTabs); }
export function rememberTabs(json) {
  openTabs = JSON.parse(json);
  try { sessionStorage.setItem('laml.open-documents', json); } catch (e) { storageError = errorText(e); }
}
export function openRecord(id) {
  const row = records.get(id);
  if (!row || row.trashed) return '';
  if (!pending.has(id)) versions.set(id, row.version);
  return JSON.stringify(row);
}
export function saveDocument(id, name, source) {
  const row = {id, name, source, trashed: false, updated: Date.now(), version: randomId()};
  records.set(id, row); pending.set(id, row); flush();
}
function flush() {
  if (saving || !pending.size || !db) return;
  saving = true;
  chain = chain.then(async () => {
    for (const [id, row] of [...pending]) {
      const tx = db.transaction('documents', 'readwrite'), done = complete(tx), store = tx.objectStore('documents');
      const old = await request(store.get(id));
      if (old && old.version !== versions.get(id) && (old.source !== row.source || old.name !== row.name || old.trashed)) {
        // Preserve the other tab's version atomically instead of silently losing it.
        store.put({...old, id: randomId(), name: old.name + ' (other tab)', updated: Date.now()});
        notice = 'Another tab changed this document. Both versions have been kept in the library.';
      }
      store.put(row); await done; versions.set(id, row.version);
      if (pending.get(id) === row) pending.delete(id);
    }
    storageError = ''; channel?.postMessage('changed'); await refresh();
  }).catch(e => { storageError = errorText(e); }).finally(() => { saving = false; if (pending.size && !storageError) flush(); });
}
export function libraryAction(action, id, name) {
  // Do not delete a buffer whose most recent automatic save is still unacknowledged.
  if (pending.size || saving) { notice = 'Wait until all changes are saved in this browser, then try again.'; return; }
  chain = chain.then(async () => {
    if (!db) throw new Error('Browser storage unavailable');
    const tx = db.transaction('documents', 'readwrite'), done = complete(tx), store = tx.objectStore('documents');
    if (action === 'clear') store.clear();
    else if (action === 'empty') {
      const rows = await request(store.getAll());
      for (const row of rows) if (row.trashed) store.delete(row.id);
    } else {
      const row = await request(store.get(id));
      if (row) {
        if (action === 'delete') store.delete(id);
        else {
          if (action === 'rename') row.name = name;
          if (action === 'trash') row.trashed = true;
          if (action === 'restore') row.trashed = false;
          const previous = row.version;
          row.updated = Date.now(); row.version = randomId(); store.put(row);
          if (versions.get(id) === previous) versions.set(id, row.version);
        }
      }
    }
    await done; actions.push({action, id, name}); storageError = ''; channel?.postMessage('changed'); await refresh();
  }).catch(e => { storageError = errorText(e); });
}
export function importLaml() {
  const input = document.createElement('input'); input.type = 'file'; input.accept = '.laml'; input.multiple = true;
  input.onchange = async () => {
    for (const file of input.files) {
      if (!file.name.toLowerCase().endsWith('.laml')) { notice = 'Only .laml files can be imported.'; continue; }
      try { imports.push({name: file.name, source: await file.text()}); }
      catch (e) { notice = `Could not read ${file.name}: ${e.message}`; }
    }
    input.remove();
  };
  input.click();
}
export function takeImports() { const out = JSON.stringify(imports); imports = []; return out; }
export function download(name, bytes, mime) {
  const url = URL.createObjectURL(new Blob([bytes], {type: mime}));
  const link = document.createElement('a'); link.href = url; link.download = name; link.click();
  setTimeout(() => URL.revokeObjectURL(url), 60000);
}

export function takeActions() { const out = JSON.stringify(actions); actions = []; return out; }
