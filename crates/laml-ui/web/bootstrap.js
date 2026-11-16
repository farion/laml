// Bootstrap only. All editor behavior is the shared Rust/egui application.
const status = document.getElementById('status');
const retry = document.getElementById('retry');
const spinner = document.getElementById('loading-spinner');
// Match the shared Rust language policy before WASM has started.
function bootLanguage() {
  let preference = 'Auto';
  try { preference = JSON.parse(localStorage.getItem('laml.editor.settings') || '{}').language || 'Auto'; } catch (_) {}
  if (preference === 'German') return 'de';
  if (preference === 'English') return 'en';
  for (const tag of navigator.languages?.length ? navigator.languages : [navigator.language || 'en']) {
    const base = tag.split(/[-_.@]/)[0].toLowerCase();
    if (base === 'de' || base === 'en') return base;
  }
  return 'en';
}
const language = bootLanguage();
document.documentElement.lang = language;
const bootText = language === 'de' ? {
  loading: 'Laden ...', reload: 'Neu laden',
  failed: 'Editor konnte nicht gestartet werden. Prüfe die Serververbindung und Grafikunterstützung und lade die Seite neu.'
} : {
  loading: 'Loading ...', reload: 'Reload',
  failed: 'Editor could not start. Check host availability and graphics support, then reload.'
};
retry.textContent = bootText.reload;
async function start() {
  retry.hidden = true;
  spinner.hidden = false;
  status.textContent = bootText.loading;
  try {
    const response = await fetch('/api/options');
    if (!response.ok) throw new Error('Host unavailable');
    const options = await response.json();
    const module = await import('/pkg/laml_ui.js');
    await module.default();
    document.getElementById('loading').hidden = true;
    const canvas = document.getElementById('editor');
    canvas.hidden = false;
    await module.start_editor(canvas, options.limits === true, options.browser === true);
    canvas.dataset.lamlBrowser = String(options.browser === true);
    canvas.focus();
  } catch (_) {
    document.getElementById('editor').hidden = true;
    document.getElementById('loading').hidden = false;
    status.textContent = bootText.failed;
    spinner.hidden = true;
    retry.hidden = false;
  }
}
// Do not mistake a hidden/frozen tab for a departed page. BFCache pages remain live.
window.addEventListener('pagehide', event => {
  if (event.persisted) return;
  if (document.getElementById('editor').dataset.lamlBrowser === 'true') return;
  const client = document.getElementById('editor').dataset.lamlClient;
  if (client) navigator.sendBeacon('/api/depart', client);
});
retry.addEventListener('click', () => location.reload());
start();
