// Focused acceptance checks for browser-local persistence and session admission.
// Run after building native/WASM debug artifacts and wasm-bindgen assets.
import fs from 'node:fs';
import {spawn} from 'node:child_process';
import {fileURLToPath} from 'node:url';
const repo=fileURLToPath(new URL('../',import.meta.url));
const httpPort=process.env.LAML_TEST_PORT || '17883';
const base='http://127.0.0.1:'+httpPort;
const root=fs.mkdtempSync('/tmp/laml-browser-smoke-');
fs.mkdirSync(root+'/host');
const host=spawn(repo+'target/debug/laml-ui',['serve','--assets',repo+'target/editor-web','--max-users','1','--bind','127.0.0.1:'+httpPort],{stdio:'ignore',cwd:root+'/host'});
for(let n=0;n<100;n++){try{if((await fetch(base+'/api/options')).ok)break;}catch{}await new Promise(r=>setTimeout(r,100));}
const chrome=spawn('google-chrome',['--headless=new','--no-sandbox','--disable-dev-shm-usage','--use-angle=swiftshader','--enable-unsafe-swiftshader','--remote-debugging-port=0','--window-size=1568,975','--user-data-dir='+root,base],{stdio:'ignore'});
const sleep=ms=>new Promise(r=>setTimeout(r,ms));let ws;
try{let port;for(let i=0;i<200;i++){if(fs.existsSync(root+'/DevToolsActivePort')){port=fs.readFileSync(root+'/DevToolsActivePort','utf8').split('\n')[0];break;}await sleep(50);}if(!port)throw Error('Chrome did not start');
let tab;for(let i=0;i<100;i++){tab=(await(await fetch('http://127.0.0.1:'+port+'/json/list')).json()).find(t=>t.type==='page');if(tab)break;await sleep(50);}
ws=new WebSocket(tab.webSocketDebuggerUrl);await new Promise((r,j)=>{ws.onopen=r;ws.onerror=j});let seq=0;const pending=new Map(),errors=[];
ws.onmessage=e=>{const m=JSON.parse(e.data);if(m.id){const p=pending.get(m.id);pending.delete(m.id);m.error?p?.reject(Error(JSON.stringify(m.error))):p?.resolve(m.result)}else if(m.method==='Runtime.exceptionThrown')errors.push(m.params.exceptionDetails.text)};
const command=(method,params={})=>new Promise((resolve,reject)=>{let id=++seq;pending.set(id,{resolve,reject});ws.send(JSON.stringify({id,method,params}))});
const evaluate=async expression=>{const r=await command('Runtime.evaluate',{expression,returnByValue:true,awaitPromise:true});if(r.exceptionDetails)throw Error(JSON.stringify(r.exceptionDetails));return r.result.value};
await command('Runtime.enable');await command('Page.enable');await command('Emulation.setDeviceMetricsOverride',{width:1568,height:975,deviceScaleFactor:1,mobile:false});
await evaluate('localStorage.setItem("laml.editor.settings", JSON.stringify({welcome_seen:true,fit_preview:true}))');
await command('Page.reload',{ignoreCache:true});
let ready=false;for(let i=0;i<600;i++){if(await evaluate('document.getElementById("editor")?.dataset.lamlPreview==="ready"')){ready=true;break;}await sleep(100);}if(!ready){ console.log(JSON.stringify({errors,dom:await evaluate('document.body.innerText'),state:await evaluate('document.getElementById("editor")?.dataset')}));throw Error('Editor preview not ready');}
const binding=fs.readFileSync(repo+'target/editor-web/laml_ui.js','utf8');
const storagePath='/pkg/'+binding.match(/['"]\.\/(snippets\/[^'"]+browser-storage\.js)['"]/)[1];
await evaluate(`import('${storagePath}').then(m=>{window.storage=m;return true})`);
const initial=await evaluate('JSON.parse(storage.snapshot())');
console.log('initialRecords',initial.records.length,'browserErrors',errors);
const session=await evaluate('storage.sessionId()');
const full=await evaluate(`fetch('/api/session',{method:'POST',headers:{'X-LAML-Session':'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb'}}).then(async r=>({status:r.status,text:await r.text()}))`);
console.log('full',full);
if(full.status!==503)throw Error('Session admission failed');
const click=async(x,y)=>{await command('Input.dispatchMouseEvent',{type:'mousePressed',x,y,button:'left',clickCount:1});await command('Input.dispatchMouseEvent',{type:'mouseReleased',x,y,button:'left',clickCount:1});await sleep(300)};
await click(105,180);
await command('Input.dispatchKeyEvent',{type:'keyDown',key:'a',code:'KeyA',modifiers:2,windowsVirtualKeyCode:65});await command('Input.dispatchKeyEvent',{type:'keyUp',key:'a',code:'KeyA',modifiers:0,windowsVirtualKeyCode:65});
const source='semantics {\n BrowserSavedNode\n}\n';
await command('Runtime.evaluate',{expression:`(()=>{const data=new DataTransfer();data.setData('text/plain',${JSON.stringify(source)});document.activeElement.dispatchEvent(new ClipboardEvent('paste',{clipboardData:data,bubbles:true,cancelable:true}));})()`});
await sleep(3000);
console.log('Source edit saved in browser');
if(!(await evaluate('storage.snapshot()')).includes('BrowserSavedNode'))throw Error('Text edit not persisted');
await command('Page.reload',{ignoreCache:true});await sleep(4000);
await evaluate(`import('${storagePath}').then(m=>{window.storage=m;return true})`);
if(!(await evaluate('storage.snapshot()')).includes('BrowserSavedNode'))throw Error('Reload lost source');
await click(28,24);await sleep(400);
let img=await command('Page.captureScreenshot',{format:'png'});fs.writeFileSync(root+'/library.png',Buffer.from(img.data,'base64'));
// Close library, edit while disconnected, then reload the invalid source.
await click(1073,356);
await command('Network.enable');
await command('Network.emulateNetworkConditions',{offline:true,latency:0,downloadThroughput:-1,uploadThroughput:-1});
await click(105,180);
await command('Input.dispatchKeyEvent',{type:'keyDown',key:'a',code:'KeyA',modifiers:2,windowsVirtualKeyCode:65});await command('Input.dispatchKeyEvent',{type:'keyUp',key:'a',code:'KeyA',modifiers:0,windowsVirtualKeyCode:65});
const invalid='semantics {\n OfflineIncomplete';
await command('Runtime.evaluate',{expression:`(()=>{const data=new DataTransfer();data.setData('text/plain',${JSON.stringify(invalid)});document.activeElement.dispatchEvent(new ClipboardEvent('paste',{clipboardData:data,bubbles:true,cancelable:true}));})()`});
await sleep(1500);
if(!(await evaluate('storage.snapshot()')).includes('OfflineIncomplete'))throw Error('Offline edit not persisted');
await command('Network.emulateNetworkConditions',{offline:false,latency:0,downloadThroughput:-1,uploadThroughput:-1});
await command('Page.reload',{ignoreCache:true});await sleep(3000);
await evaluate(`import('${storagePath}').then(m=>{window.storage=m;return true})`);
const data=JSON.parse(await evaluate('storage.snapshot()'));
if(!data.records[0].source.includes('OfflineIncomplete'))throw Error('Invalid source lost on reload');
const id=data.records[0].id;
await evaluate(`storage.libraryAction('rename','${id}','Renamed.laml')`);await sleep(500);
if(!JSON.parse(await evaluate('storage.snapshot()')).records.some(r=>r.name==='Renamed.laml'))throw Error('Rename failed');
// Real Ctrl+S path downloads markup, not a host save request.
fs.mkdirSync(root+'/downloads');
await command('Browser.setDownloadBehavior',{behavior:'allow',downloadPath:root+'/downloads'});
await command('Input.dispatchKeyEvent',{type:'keyDown',key:'s',code:'KeyS',modifiers:2,windowsVirtualKeyCode:83});
await command('Input.dispatchKeyEvent',{type:'keyUp',key:'s',code:'KeyS',modifiers:0,windowsVirtualKeyCode:83});await sleep(1000);
if(!fs.existsSync(root+'/downloads/Renamed.laml'))throw Error('LAML download missing');
if(!fs.readFileSync(root+'/downloads/Renamed.laml','utf8').includes('OfflineIncomplete'))throw Error('Wrong downloaded source');
await evaluate(`storage.libraryAction('trash','${id}','')`);await sleep(500);
if(!JSON.parse(await evaluate('storage.snapshot()')).records.find(r=>r.id===id)?.trashed)throw Error('Trash failed');
await evaluate(`storage.libraryAction('restore','${id}','')`);await sleep(500);
if(JSON.parse(await evaluate('storage.snapshot()')).records.find(r=>r.id===id)?.trashed)throw Error('Restore failed');
// A second browser tab must share admission, not consume another place.
const other = await command('Target.createTarget',{url:base});
await sleep(2500);
const target2=(await(await fetch('http://127.0.0.1:'+port+'/json/list')).json()).find(t=>t.id===other.targetId);
const ws2=new WebSocket(target2.webSocketDebuggerUrl);await new Promise((r,j)=>{ws2.onopen=r;ws2.onerror=j});
let seq2=0;const waiting2=new Map();
ws2.onmessage=e=>{const m=JSON.parse(e.data);if(m.id){const p=waiting2.get(m.id);waiting2.delete(m.id);m.error?p.reject(Error(JSON.stringify(m.error))):p.resolve(m.result)}};
const command2=(method,params={})=>new Promise((resolve,reject)=>{const id=++seq2;waiting2.set(id,{resolve,reject});ws2.send(JSON.stringify({id,method,params}))});
const eval2=async expression=>{const r=await command2('Runtime.evaluate',{expression,returnByValue:true,awaitPromise:true});if(r.exceptionDetails)throw Error(JSON.stringify(r.exceptionDetails));return r.result.value};
try {
await eval2(`import('${storagePath}').then(m=>{window.storage=m;return true})`);
for(let n=0;n<100;n++){if(await eval2('storage.sessionId()'))break;await sleep(100)}
if(await eval2('storage.sessionId()')!==session)throw Error('Browser tabs do not share session');
if(!await eval2('storage.available()'))throw Error('Second tab denied admission');
const race=await evaluate('storage.newId()');
await evaluate(`storage.saveDocument('${race}','Concurrent.laml','base')`);await sleep(500);
await eval2(`storage.openRecord('${race}')`);
await evaluate(`storage.openRecord('${race}');storage.saveDocument('${race}','Concurrent.laml','first-tab-version')`);await sleep(500);
await eval2(`storage.saveDocument('${race}','Concurrent.laml','second-tab-version')`);await sleep(700);
const copies=JSON.parse(await evaluate('storage.snapshot()')).records;
if(!copies.some(r=>r.source==='first-tab-version')||!copies.some(r=>r.source==='second-tab-version'))throw Error('Concurrent tab source lost');
} finally { ws2.close();await command('Target.closeTarget',{targetId:other.targetId}); }
// Failed storage is visible and retains the unsaved text for retry.
const failureId=await evaluate('storage.newId()');
await evaluate(`window.originalTx=IDBDatabase.prototype.transaction;IDBDatabase.prototype.transaction=function(){throw new Error('Simulated quota failure')};storage.saveDocument('${failureId}','Pending.laml','must-survive')`);
await sleep(500);
if(!await evaluate('storage.storageFailed() && !storage.safeToClose()'))throw Error('Storage failure not surfaced');
await evaluate(`IDBDatabase.prototype.transaction=window.originalTx;storage.saveDocument('${failureId}','Pending.laml','must-survive')`);await sleep(800);
if(!await evaluate('storage.safeToClose()'))throw Error('Storage write did not recover');
await evaluate(`storage.libraryAction('clear','','')`);await sleep(800);
if(JSON.parse(await evaluate('storage.snapshot()')).records.length)throw Error('Delete everything left documents');
if(fs.readdirSync(root+'/host').length)throw Error('Host created user-data files');
console.log(JSON.stringify({ready,reloaded:true,offline:true,invalidText:true,download:true,trash:true,clear:true,sharedTabs:true,conflictPreserved:true,storageFailure:true,errors,hostFiles:fs.readdirSync(root+'/host')}));
}finally{ws?.close();chrome.kill();host.kill();}
