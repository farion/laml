// Focused responsive-editor smoke: window widths, flyout, pane switch, tabs and touch.
// Usage: node scripts/test-editor-responsive.mjs [origin]
import fs from 'node:fs';
import {spawn} from 'node:child_process';
import assert from 'node:assert/strict';

const origin = process.argv[2] || 'http://laml-clipboard.test:7878';
const root = fs.mkdtempSync('/tmp/laml-responsive-');
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
    const id = ++sequence; const timeout=setTimeout(()=>{pending.delete(id);reject(Error('CDP timeout '+method));},20000); pending.set(id, {resolve:r=>{clearTimeout(timeout);resolve(r)}, reject:e=>{clearTimeout(timeout);reject(e)}});
    ws.send(JSON.stringify({id, method, params}));
  });
  async function evaluate(expression) {
    const result = await command('Runtime.evaluate', {expression, returnByValue: true, awaitPromise: true});
    if (result.exceptionDetails) throw Error(JSON.stringify(result.exceptionDetails));
    return result.result.value;
  }
  async function click(x, y) {
    for (const type of ['mouseMoved', 'mousePressed', 'mouseReleased']) {
      await command('Input.dispatchMouseEvent', {type, x, y, button: 'left', clickCount: 1}); await sleep(60);
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

  const source = 'semantics {\n A\n B\n A -> B : request\n}\n';
  await evaluate(`localStorage.setItem('laml.editor.settings',JSON.stringify({welcome_seen:true})); Promise.all(Array.from({length:8},(_,i)=>storage.saveDocument('responsive-'+i,'Responsive-'+i+'.laml',${JSON.stringify(source)}))).then(()=>storage.rememberTabs(JSON.stringify(Array.from({length:8},(_,i)=>'responsive-'+i))))`);
  await wait(() => evaluate('storage.safeToClose()'), 'saved fixture');
  await command('Page.addScriptToEvaluateOnNewDocument', {source:`sessionStorage.setItem('laml.open-documents',JSON.stringify(Array.from({length:8},(_,i)=>'responsive-'+i)))`});
  await command('Page.reload', {ignoreCache:true});
  await wait(() => evaluate('document.title.includes("Responsive-")'), 'document');
  await sleep(1000);
  for (const width of [1568,1000,800,600,390,320]) {
    await command('Emulation.setDeviceMetricsOverride', {width,height:850,deviceScaleFactor:1,mobile:false});
    await sleep(350);
    await screenshot('width-'+width);
  }
  await command('Emulation.setDeviceMetricsOverride', {width:390,height:850,deviceScaleFactor:1,mobile:true});
  await sleep(400);
  await click(357,104);
  await screenshot('mobile-properties');
  await click(358,104);
  await click(55,104);
  await screenshot('mobile-pane-menu');
  await click(55,146);
  await screenshot('mobile-source');
  await click(55,104);
  await click(55,185);
  await screenshot('mobile-diagram-again');
  await click(374,62);
  await screenshot('mobile-tabs-scrolled');
  await click(370,829);
  await screenshot('mobile-footer-menu');
  await click(180,300);
  await command('Emulation.setTouchEmulationEnabled',{enabled:true,maxTouchPoints:1});
  await command('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[{x:95,y:474}]});
  await sleep(80);
  await command('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});
  await sleep(300);
  await screenshot('mobile-touch-selected');
  await command('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[{x:95,y:300}]});
  await sleep(80);
  await command('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});
  await sleep(300);
  await screenshot('mobile-touch-cleared');
  await evaluate(`import('${storagePath}').then(m => {window.storage=m;return true})`);
  const records = await evaluate(`JSON.parse(storage.snapshot()).records`);
  assert.equal(records.length,8);
  assert.ok(records.every(r=>r.source===source),'Responsive navigation must preserve source');
  console.log(JSON.stringify({root,records:records.length,sourceUnchanged:true,errors}));
  assert.equal(errors.length,0);
} finally { ws?.close(); chrome.kill('SIGTERM'); }
