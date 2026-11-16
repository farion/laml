// Focused language acceptance against an already running debug browser host.
// Usage: node scripts/test-editor-language.mjs [http://127.0.0.1:7878]
import fs from 'node:fs';
import {spawn} from 'node:child_process';
import assert from 'node:assert/strict';

const origin = process.argv[2] || 'http://127.0.0.1:7878';
const root = fs.mkdtempSync('/tmp/laml-language-');
const chrome = spawn(process.env.CHROME_BIN || 'google-chrome', [
  '--headless=new', '--no-sandbox', '--disable-dev-shm-usage', '--use-angle=swiftshader',
  '--enable-unsafe-swiftshader', '--remote-debugging-port=0', '--window-size=1568,975',
  '--user-data-dir=' + root, origin,
], {stdio: 'ignore'});
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
let ws;
try {
  async function wait(test, description) {
    for (let n = 0; n < 600; n++) {
      if (await test()) return;
      await sleep(100);
    }
    throw Error('Timed out: ' + description);
  }
  await wait(() => fs.existsSync(root + '/DevToolsActivePort'), 'Chrome');
  const port = fs.readFileSync(root + '/DevToolsActivePort', 'utf8').split('\n')[0];
  let tab;
  await wait(async () => {
    tab = (await (await fetch(`http://127.0.0.1:${port}/json/list`)).json()).find(t => t.type === 'page');
    return !!tab;
  }, 'page');
  ws = new WebSocket(tab.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => { ws.onopen = resolve; ws.onerror = reject; });
  let sequence = 0;
  const pending = new Map(), errors = [];
  ws.onmessage = event => {
    const message = JSON.parse(event.data);
    if (message.id) {
      const p = pending.get(message.id); pending.delete(message.id);
      if (message.error) p?.reject(Error(JSON.stringify(message.error)));
      else p?.resolve(message.result);
    } else if (message.method === 'Runtime.exceptionThrown') {
      errors.push(message.params.exceptionDetails.text);
    }
  };
  const command = (method, params = {}) => new Promise((resolve, reject) => {
    const id = ++sequence; pending.set(id, {resolve, reject});
    ws.send(JSON.stringify({id, method, params}));
  });
  async function evaluate(expression) {
    const result = await command('Runtime.evaluate', {expression, returnByValue: true, awaitPromise: true});
    if (result.exceptionDetails) throw Error(JSON.stringify(result.exceptionDetails));
    return result.result.value;
  }
  async function click(x, y) {
    for (const type of ['mouseMoved', 'mousePressed', 'mouseReleased']) {
      await command('Input.dispatchMouseEvent', {type, x, y, button: 'left', clickCount: 1});
    }
    await sleep(250);
  }
  async function screenshot(name) {
    const shot = await command('Page.captureScreenshot', {format: 'png'});
    fs.writeFileSync(root + '/' + name + '.png', Buffer.from(shot.data, 'base64'));
  }
  async function reload(language, loading) {
    await command('Page.reload', {ignoreCache: true});
    await wait(() => evaluate('document.getElementById("loading")?.hidden === true'), 'WASM startup');
    await sleep(400);
    assert.equal(await evaluate('document.documentElement.lang'), language);
    assert.equal(await evaluate('document.getElementById("status").textContent'), loading);
    assert.equal(await evaluate('document.title'), language === 'de'
      ? 'Keine Datei geöffnet — LAML Editor' : 'No file open — LAML Editor');
  }
  await command('Runtime.enable'); await command('Page.enable');
  await command('Emulation.setDeviceMetricsOverride', {width: 1568, height: 975, deviceScaleFactor: 1, mobile: false});
  let script;
  async function locales(tags) {
    if (script) await command('Page.removeScriptToEvaluateOnNewDocument', {identifier: script});
    const result = await command('Page.addScriptToEvaluateOnNewDocument', {source: `
      Object.defineProperty(navigator, 'languages', {get: () => ${JSON.stringify(tags)}});
      Object.defineProperty(navigator, 'language', {get: () => ${JSON.stringify(tags[0])}});
    `});
    script = result.identifier;
  }
  await wait(() => evaluate('document.getElementById("loading")?.hidden === true'), 'initial startup');
  await evaluate(`localStorage.setItem('laml.editor.settings', JSON.stringify({editor_theme:'Dark', welcome_seen:true}))`);
  await locales(['de-CH', 'en-US']);
  await reload('de', 'Laden ...');
  await screenshot('auto-german');
  await click(1440, 22); // Settings.
  await click(710, 399);
  await screenshot('language-dropdown-german');
  await click(690, 467); // Englisch.
  await wait(() => evaluate('JSON.parse(localStorage.getItem("laml.editor.settings")).language === "English"'), 'English selection saved');
  assert.equal(await evaluate('document.documentElement.lang'), 'en');
  await screenshot('manual-english');
  await reload('en', 'Loading ...'); // German browser must not override a manual choice.
  await click(1440, 22); await click(710, 399); await click(690, 523); // German.
  await wait(() => evaluate('JSON.parse(localStorage.getItem("laml.editor.settings")).language === "German"'), 'German selection saved');
  await locales(['en-US']);
  await reload('de', 'Laden ...');
  await click(1440, 22); await click(710, 399); await click(690, 435); // Auto.
  await wait(() => evaluate('JSON.parse(localStorage.getItem("laml.editor.settings")).language === "Auto"'), 'Auto selection saved');
  assert.equal(await evaluate('document.documentElement.lang'), 'en');
  await locales(['fr-FR']); await reload('en', 'Loading ...');
  await locales(['fr-FR', 'de-DE']); await reload('de', 'Laden ...');
  await screenshot('supported-secondary-language');
  assert.deepEqual(errors, []);
  console.log(JSON.stringify({autoDetection: true, manualOverride: true, persistedAcrossReload: true, unsupportedFallback: true, browserErrors: errors, screenshots: root}));
} finally {
  ws?.close(); chrome.kill();
}
