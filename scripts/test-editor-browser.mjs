// Real browser acceptance of the shared Rust editor, with opt-in policies.
import fs from 'node:fs';
import path from 'node:path';
import {spawn} from 'node:child_process';
import assert from 'node:assert/strict';
const limited=process.argv.includes('--limits');
const root=fs.mkdtempSync(path.resolve('target/browser-editor-'));
const workspace=path.join(root,'workspace');fs.mkdirSync(workspace);
const source='semantics {\n A\n B\n A -> B : request\n}\n';
fs.writeFileSync(path.join(workspace,'sample.laml'),source);
const log=fs.openSync(path.join(root,'host.log'),'a');
let origin='',hostText='',chrome,ws;
const host=spawn(path.resolve('target/release/laml-ui'),['serve','--workspace',workspace,'--assets',path.resolve('target/editor-web'),'--state-dir',path.join(root,'state'),'--bind','127.0.0.1:0',...(limited?['--limits','--parallelism','1','--http-checks']:[])],{stdio:['ignore',log,'pipe']});
host.stderr.on('data',bytes=>{fs.writeSync(log,bytes);hostText+=bytes;origin=hostText.match(/LAML host: (http:\/\/[^\s]+)/)?.[1]||origin;});
const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));
let captureFailure=async()=>{};
async function wait(test,description,timeout=45000){const end=Date.now()+timeout;while(Date.now()<end){try{if(await test())return;}catch(error){if(!/Chrome command failed|Browser evaluation failed/.test(error.message))throw error;}await sleep(50);}await captureFailure();throw Error('Timed out: '+description);}
try {
 await wait(()=>origin,'host start');
 const headers={'Content-Type':'application/json','Origin':origin,'X-LAML-Protocol':'1'};
 const options=await(await fetch(origin+'/api/options')).json();
 assert.equal(options.limits,limited);assert.equal(options.parallelism,limited?1:null);assert.equal(options.http_checks,limited);
 const page=await fetch(origin);assert.equal(page.headers.get('set-cookie'),null);
 let id=1;
 async function rpc(operation,revision=0){const response=await fetch(origin+'/api/editor',{method:'POST',headers,body:JSON.stringify({stamp:{version:1,session:123,document:1,request:id++,revision},operation})});assert.equal(response.status,200);assert.equal(response.headers.get('set-cookie'),null);return(await response.json()).result;}
 assert.equal((await rpc({type:'open_source',address:'sample.laml'})).Ok.data.source,source);
 for(const address of ['../outside.laml','/etc/passwd','.git/config','C:\\private\\data.laml'])assert.ok((await rpc({type:'open_source',address})).Err);
 const large='x'.repeat(4*1024*1024+1);fs.writeFileSync(path.join(workspace,'large.laml'),large);
 const largeResult=await rpc({type:'open_source',address:'large.laml'});
 if(limited)assert.ok(largeResult.Err);else assert.equal(largeResult.Ok.data.source.length,large.length);
 assert.ok((await rpc({type:'compile',source,address:'sample.laml',presentation:'Editor'})).Ok.data.Ok);
 assert.ok((await rpc({type:'save_source',address:'saved.laml',source,expected:null},1)).Ok);
 assert.ok((await rpc({type:'save_source',address:'saved.laml',source:'not applied',expected:null},2)).Err);
 const prepared=(await rpc({type:'begin_export',address:'output.svg',source_address:'sample.laml',source,format:'Svg'},2)).Ok;
 const ticket=prepared.data?.ticket??prepared.ticket;
 assert.ok((await rpc({type:'commit_export',ticket},2)).Ok);
 assert.ok(fs.readFileSync(path.join(workspace,'output.svg'),'utf8').includes('<svg'));
 const cross=await fetch(origin+'/api/editor',{method:'POST',headers:{...headers,Origin:'https://other.example'},body:JSON.stringify({stamp:{version:1,session:124,document:1,request:1,revision:0},operation:{type:'open_source',address:'sample.laml'}})});
 assert.equal(cross.status,limited?403:200);
 console.log('Policy defaults/opt-in, scoped paths, checked saves and export tickets passed.');
 chrome=spawn(process.env.CHROME_BIN||'google-chrome',['--headless=new','--no-sandbox','--disable-dev-shm-usage','--use-angle=swiftshader','--enable-unsafe-swiftshader','--remote-debugging-port=0','--window-size=1568,975','--user-data-dir='+path.join(root,'chrome'),origin],{stdio:'ignore'});
 const portFile=path.join(root,'chrome','DevToolsActivePort');await wait(()=>fs.existsSync(portFile),'Chrome start');
 const port=fs.readFileSync(portFile,'utf8').split('\n')[0];
 let tab;await wait(async()=>{tab=(await(await fetch(`http://127.0.0.1:${port}/json/list`)).json()).find(t=>t.type==='page');return!!tab;},'Chrome page');
 ws=new WebSocket(tab.webSocketDebuggerUrl);await new Promise((resolve,reject)=>{ws.onopen=resolve;ws.onerror=reject;});
 const pending=new Map(),browserErrors=[];let sequence=1;
 ws.onmessage=event=>{const message=JSON.parse(event.data);if(message.id){const p=pending.get(message.id);pending.delete(message.id);if(message.error)p?.reject(Error('Chrome command failed'));else p?.resolve(message.result);}else if(message.method==='Runtime.exceptionThrown')browserErrors.push(message.params.exceptionDetails.text);else if(message.method==='Page.javascriptDialogOpening')command('Page.handleJavaScriptDialog',{accept:true}).catch(()=>{});};
 function command(method,params={}){const id=sequence++;return new Promise((resolve,reject)=>{pending.set(id,{resolve,reject});ws.send(JSON.stringify({id,method,params}));});}
 async function evaluate(expression){const result=await command('Runtime.evaluate',{expression,returnByValue:true,awaitPromise:true});if(result.exceptionDetails)throw Error('Browser evaluation failed');return result.result.value;}
 async function screenshot(name){const image=await command('Page.captureScreenshot',{format:'png'});fs.writeFileSync(path.join(root,name),Buffer.from(image.data,'base64'));}
 captureFailure=async()=>{await screenshot('failure.png');console.log('Failure evidence:',path.relative(process.cwd(),root));console.log(await evaluate('JSON.stringify({title:document.title,state:document.readyState,visibility:document.visibilityState,canvas:{...document.getElementById("editor")?.dataset},status:document.getElementById("status")?.textContent})'));console.log('Browser errors:',browserErrors);};
 await command('Runtime.enable');await command('Network.enable');await command('Page.enable');
 await command('Emulation.setDeviceMetricsOverride',{width:1568,height:975,deviceScaleFactor:1,mobile:false});
 await wait(()=>evaluate('document.getElementById("editor")?.dataset.lamlPreview==="ready"'),'shared WASM editor',60000);
 assert.equal((await command('Network.getCookies',{urls:[origin]})).cookies.length,0);
 async function click(x,y){await command('Input.dispatchMouseEvent',{type:'mouseMoved',x,y});await command('Input.dispatchMouseEvent',{type:'mousePressed',x,y,button:'left',clickCount:1});await command('Input.dispatchMouseEvent',{type:'mouseReleased',x,y,button:'left',clickCount:1});await sleep(150);}
 async function key(key,code,modifiers=0){const vk=code.startsWith('Key')?code.charCodeAt(3):({Escape:27,Enter:13,Tab:9}[code]||13);await command('Input.dispatchKeyEvent',{type:'keyDown',key,code,modifiers,windowsVirtualKeyCode:vk,nativeVirtualKeyCode:vk});await command('Input.dispatchKeyEvent',{type:'keyUp',key,code,modifiers:0,windowsVirtualKeyCode:vk,nativeVirtualKeyCode:vk});await sleep(100);}
 async function paste(text){await evaluate(`(()=>{const data=new DataTransfer();data.setData('text/plain',${JSON.stringify(text)});document.activeElement.dispatchEvent(new ClipboardEvent('paste',{clipboardData:data,bubbles:true,cancelable:true}));})()`);await sleep(100);}
 await sleep(1000);await click(80,24);await sleep(400);await click(680,497);await command('Input.insertText',{text:'sample.laml'});await click(632,564);
 await wait(()=>evaluate('document.title.startsWith("sample.laml") && document.getElementById("editor").dataset.lamlPreview==="ready"'),'open server file');await sleep(500);
 const changed=source+'// browser edit\n';
 await click(60,166);await key('a','KeyA',2);await paste(changed);await sleep(700);await key('s','KeyS',2);
 await wait(()=>fs.readFileSync(path.join(workspace,'sample.laml'),'utf8')===changed,'save source');
 await key('z','KeyZ',2);await sleep(500);await key('s','KeyS',2);await wait(()=>fs.readFileSync(path.join(workspace,'sample.laml'),'utf8')===source,'shared undo');
 await key('Z','KeyZ',10);await sleep(500);await key('s','KeyS',2);await wait(()=>fs.readFileSync(path.join(workspace,'sample.laml'),'utf8')===changed,'shared redo');
 await click(742,537);const beforeDrag=await evaluate('Number(document.getElementById("editor").dataset.lamlRevision)');
 await command('Input.dispatchMouseEvent',{type:'mouseMoved',x:742,y:537});await command('Input.dispatchMouseEvent',{type:'mousePressed',x:742,y:537,button:'left',clickCount:1});
 for(let step=1;step<=8;step++){await command('Input.dispatchMouseEvent',{type:'mouseMoved',x:742+(1114-742)*step/8,y:537,button:'left',buttons:1});await sleep(60);}
 await command('Input.dispatchMouseEvent',{type:'mouseReleased',x:1114,y:537,button:'left',clickCount:1});
 await wait(()=>evaluate(`Number(document.getElementById("editor").dataset.lamlRevision)>${beforeDrag} && document.getElementById("editor").dataset.lamlPreview==="ready"`),'diagram connection preflight');await sleep(700);await key('s','KeyS',2);
 await wait(()=>(fs.readFileSync(path.join(workspace,'sample.laml'),'utf8').match(/A -> B/g)||[]).length===2,'save diagram edit');
 await click(742,537);const beforeLabel=await evaluate('Number(document.getElementById("editor").dataset.lamlRevision)');
 await click(1460,223);await key('a','KeyA',2);await paste('Browser A');await key('Enter','Enter');
 await wait(()=>evaluate(`Number(document.getElementById("editor").dataset.lamlRevision)>${beforeLabel} && document.getElementById("editor").dataset.lamlPreview==="ready"`),'inspector edit');await key('s','KeyS',2);
 await wait(()=>fs.readFileSync(path.join(workspace,'sample.laml'),'utf8').includes('Browser A'),'save inspector edit');
 for(const [ext,y,signature]of[['svg',143,Buffer.from('<svg')],['png',185,Buffer.from([137,80,78,71])],['pdf',227,Buffer.from('%PDF')]]){
   await sleep(500);await click(430,105);await click(418,y);await sleep(300);await click(680,497);await key('a','KeyA',2);await command('Input.insertText',{text:'browser.'+ext});await click(650,564);
   await wait(()=>fs.existsSync(path.join(workspace,'browser.'+ext)),ext+' export');
   const data=fs.readFileSync(path.join(workspace,'browser.'+ext));assert.ok(ext==='svg'?data.includes(signature):data.subarray(0,signature.length).equals(signature));
 }
 console.log('Browser open/edit, shared undo/redo, diagram drag, inspector and SVG/PNG/PDF passed.');
 const retained=fs.readFileSync(path.join(workspace,'sample.laml'),'utf8');
 const recoverySource=retained+'// recover after browser reload\n';
 await click(60,166);await key('a','KeyA',2);await paste(recoverySource);
 function snapshotReady(){try{return fs.readdirSync(path.join(root,'state','recovery')).filter(f=>f.endsWith('.json')).some(f=>JSON.parse(fs.readFileSync(path.join(root,'state','recovery',f),'utf8')).source===recoverySource);}catch{return false;}}
 await wait(snapshotReady,'host recovery acknowledgement');
 await sleep(1500);
 // Pause real browser execution: no polling. Another client must not evict its worker.
 await command('Debugger.enable');
 await command('Debugger.pause');
 await sleep(12500);
 const duringPause=await rpc({type:'recovery_list'},2);
 assert.ok(duringPause.Ok);
 assert.ok(!duringPause.Ok.data.some(record=>record.source===recoverySource),'inactive page snapshot must stay leased');
 await command('Debugger.resume');
 await command('Debugger.disable');
 await sleep(2000);
 assert.equal(await evaluate('Number(document.getElementById("editor").dataset.lamlRecoveryCount)'),0);
 assert.ok(snapshotReady(),'inactivity must preserve the unsaved snapshot');
 console.log('Paused browser >12 seconds plus another client: no false recovery.');
 // A later orphan is offered on the next opening, not during the editing session.
 const laterSource='semantics {\n LaterOrphan\n}\n';
 assert.ok((await rpc({type:'recovery_sync',address:null,source:laterSource,dirty:true},2)).Ok);
 assert.equal((await fetch(origin+'/api/depart',{method:'POST',headers,body:'123'})).status,204);
 let laterRecord;
 await wait(async()=>{const reply=await rpc({type:'recovery_list'},2);laterRecord=reply.Ok?.data.find(record=>record.source===laterSource);return !!laterRecord;},'later orphan available on host');
 await sleep(12500);
 assert.equal(await evaluate('Number(document.getElementById("editor").dataset.lamlRecoveryCount)'),0,'editing continues after a later orphan appears');
 assert.ok((await rpc({type:'recovery_discard',id:laterRecord.file},2)).Ok);
 console.log('Recovery discovery is tied to editor opening; later orphans are retained for the next opening.');
 const previousClient=await evaluate('document.getElementById("editor").dataset.lamlClient');
 await command('Page.reload');await command('Page.bringToFront');await wait(()=>evaluate(`document.getElementById("editor")?.dataset.lamlClient!==${JSON.stringify(previousClient)} && document.getElementById("editor")?.dataset.lamlPreview==="ready"`),'reload editor');
 await wait(()=>evaluate('Number(document.getElementById("editor").dataset.lamlRecoveryCount)>0'),'abandoned host snapshot');await screenshot('recovery.png');
 assert.equal(fs.readFileSync(path.join(workspace,'sample.laml'),'utf8'),retained);
 await click(572,545);await wait(()=>evaluate('document.title.startsWith("sample.laml*") && document.getElementById("editor").dataset.lamlPreview==="ready"'),'restore unsaved buffer');
 assert.equal(fs.readFileSync(path.join(workspace,'sample.laml'),'utf8'),retained);await sleep(1500);await key('s','KeyS',2);
 await wait(()=>fs.readFileSync(path.join(workspace,'sample.laml'),'utf8')===recoverySource,'explicit recovered save');
 assert.equal((await command('Network.getCookies',{urls:[origin]})).cookies.length,0);
 assert.equal(browserErrors.length,0);await screenshot('editor.png');
 console.log('Reload/recovery without cookies passed. Mode:',limited?'opt-in limits/checks/parallelism':'unrestricted defaults');
 console.log('Evidence:',path.relative(process.cwd(),root));
}finally{ws?.close();chrome?.kill();host.kill();fs.closeSync(log);}
