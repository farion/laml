// Bootstrap only. All editor behavior is the shared Rust/egui application.
const status = document.getElementById('status');
const retry = document.getElementById('retry');
const spinner = document.getElementById('loading-spinner');
async function start() {
  retry.hidden = true;
  spinner.hidden = false;
  status.textContent = 'Loading editor…';
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
    status.textContent = 'Editor could not start. Check host availability and graphics support, then reload.';
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
