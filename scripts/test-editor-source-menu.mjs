// Focused source context-menu acceptance, including real HTTP clipboard transport.
// Usage: node scripts/test-editor-source-menu.mjs [origin]
import fs from 'node:fs';
import {spawn} from 'node:child_process';
import assert from 'node:assert/strict';

const origin = process.argv[2] || 'http://laml-clipboard.test:7878';
const root = fs.mkdtempSync('/tmp/laml-clipboard-');
const chrome = spawn(process.env.CHROME_BIN || 'google-chrome', [
  '--headless=new', '--no-proxy-server', '--host-resolver-rules=MAP laml-clipboard.test 127.0.0.1', '--no-sandbox', '--disable-dev-shm-usage', '--use-angle=swiftshader',
  '--enable-unsafe-swiftshader', '--remote-debugging-port=0', '--window-size=1568,975',
  '--user-data-dir=' + root, origin,
], {stdio: 'ignore'});
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
let ws;
try {
  async function wait(test, description) {
    for (let n = 0; n < 600; n++) {
      try { if (await test()) return; }
      catch (error) {
        if (!/Inspected target navigated or closed|Execution context was destroyed|Cannot find context/.test(String(error))) throw error;
      }
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
  await command('Runtime.enable'); await command('Page.enable');
  await command('Emulation.setDeviceMetricsOverride', {width:1568,height:975,deviceScaleFactor:1,mobile:false});
  await wait(() => evaluate('document.getElementById("loading")?.hidden === true'), 'startup');
  const secure = await evaluate('window.isSecureContext');
  if (secure) await command('Browser.grantPermissions',{origin,permissions:['clipboardReadWrite','clipboardSanitizedWrite']});
  const binding = fs.readFileSync('target/editor-web/laml_ui.js', 'utf8');
  const storagePath = '/pkg/' + binding.match(/['"]\.\/(snippets\/[^'"]+browser-storage\.js)['"]/)[1];
  await evaluate(`import('${storagePath}').then(m => {window.storage=m;return true})`);
  const source = '// Grüße 日本語\n';
  await evaluate(`localStorage.setItem('laml.editor.settings',JSON.stringify({welcome_seen:true}));storage.saveDocument('clipboard-test','Clipboard.laml',${JSON.stringify(source)});storage.rememberTabs('["clipboard-test"]')`);
  await wait(() => evaluate('storage.safeToClose()'), 'saved fixture');
  // Seed tab restoration on the new page, not between two empty-workspace frames.
  await command('Page.addScriptToEvaluateOnNewDocument', {source:`sessionStorage.setItem('laml.open-documents','["clipboard-test"]')`});
  await command('Page.reload', {ignoreCache:true});
  await sleep(400);
  await wait(() => evaluate('document.title.includes("Clipboard.laml")'), 'document');
  await evaluate(`import('${storagePath}').then(m => {window.storage=m;return true})`);
  await evaluate(`window.copies=[];for(const kind of ['copy','cut']) document.addEventListener(kind,e=>copies.push({kind,text:e.clipboardData.getData('text/plain'),trusted:e.isTrusted}))`);
  await sleep(1200);
  await click(140,150);
  async function key(letter, modifiers=2) {
    for (const type of ['keyDown','keyUp']) await command('Input.dispatchKeyEvent', {
      type,key:letter,code:'Key'+letter.toUpperCase(),modifiers,
      windowsVirtualKeyCode:letter.toUpperCase().charCodeAt(0)
    });
    await sleep(200);
  }
  async function sourceIs(text) {
    await wait(() => evaluate(`JSON.parse(storage.snapshot()).records.find(r=>r.id==='clipboard-test').source === ${JSON.stringify(text)}`), 'source '+JSON.stringify(text));
  }
  async function menu(index) {
    const x=140,y=150;
    for(const type of ['mouseMoved','mousePressed','mouseReleased']) await command('Input.dispatchMouseEvent',{type,x,y,button:'right',clickCount:1});
    await sleep(300);
    await screenshot('menu');
    await click(x+65,y+22+34*index);
    await sleep(300);
  }
  await key('a'); await menu(0); // Copy selection.
  await sourceIs(source);
  await menu(3); // Delete, without replacing the clipboard.
  await sourceIs('');
  await key('v'); await sourceIs(source); // The menu really copied to OS clipboard.
  await key('a'); await menu(2); // Cut selection.
  await sourceIs('');
  await menu(1); // Paste via API on secure origin, or safe keyboard hint on HTTP.
  await sourceIs(secure ? source : '');
  if(!secure) { await key('v'); await sourceIs(source); }
  await key('z'); await sourceIs('');
  await key('z'); await sourceIs(source);
  assert.deepEqual(errors, []);
  console.log(JSON.stringify({secure,copy:true,cut:true,delete:true,paste:secure?'menu':'keyboard fallback',selectionPreserved:true,undo:true,errors,screenshots:root}));
} finally { ws?.close(); chrome.kill(); }
