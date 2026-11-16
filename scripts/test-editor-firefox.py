#!/usr/bin/env python3
"""Firefox regression for strict HTTP checks (requires Firefox/geckodriver).

Build target/debug/laml-ui and target/editor-web first, then run this script.
Uses synthetic source and a temporary browser profile; no saved user documents.
"""
import json
import socket
import subprocess
import tempfile
import time
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def free_port():
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0))
        return sock.getsockname()[1]


def request(url, data=None, method=None, headers=None):
    if data is not None:
        data = json.dumps(data).encode()
    req = urllib.request.Request(url, data=data, method=method, headers=headers or {})
    try:
        return urllib.request.urlopen(req, timeout=90)
    except urllib.error.HTTPError as error:
        return error


def wait(url, process):
    for _ in range(200):
        if process.poll() is not None:
            raise RuntimeError('Test process exited during startup')
        try:
            with request(url) as response:
                if response.status == 200:
                    return
        except OSError:
            pass
        time.sleep(0.1)
    raise RuntimeError('Timed out starting ' + url)


def main():
    origin = f'http://127.0.0.1:{free_port()}'
    driver_url = f'http://127.0.0.1:{free_port()}'
    processes = []
    session = None
    with tempfile.TemporaryDirectory(prefix='laml-firefox-') as work:
        with open(Path(work) / 'processes.log', 'w+') as log:
            try:
                host = subprocess.Popen([
                    str(ROOT / 'target/debug/laml-ui'), 'serve',
                    '--assets', str(ROOT / 'target/editor-web'),
                    '--bind', origin.removeprefix('http://'), '--http-checks',
                    '--origin', origin,
                ], cwd=work, stdout=log, stderr=log)
                processes.append(host)
                driver = subprocess.Popen([
                    'geckodriver', '--port', driver_url.rsplit(':', 1)[1],
                ], cwd=work, stdout=log, stderr=log)
                processes.append(driver)
                wait(origin + '/api/options', host)
                wait(driver_url + '/status', driver)

                def webdriver(path, data=None, method=None):
                    with request(driver_url + path, data, method, {
                        'Content-Type': 'application/json',
                    }) as response:
                        result = json.load(response)
                        assert response.status == 200, result
                        return result['value']

                value = webdriver('/session', {'capabilities': {'alwaysMatch': {
                    'browserName': 'firefox',
                    'moz:firefoxOptions': {'args': ['-headless']},
                }}})
                session = '/session/' + value['sessionId']
                webdriver(session + '/url', {'url': origin})
                result = webdriver(session + '/execute/async', {
                    'script': '''
const done = arguments[arguments.length - 1];
(async () => {
    const headers = {'Content-Type':'application/json', 'X-LAML-Protocol':'1',
                     'X-LAML-Session':'f'.repeat(32)};
    let id = 0;
    async function rpc(operation) {
        const r = await fetch('/api/editor', {
            method:'POST', mode:'same-origin', headers,
            body:JSON.stringify({stamp:{version:1,session:1,document:1,
                request:++id,revision:0},operation})
        });
        return {status:r.status, body:r.ok ? await r.json() : null};
    }
    const source = 'semantics {\\n FirefoxA\\n FirefoxB\\n FirefoxA -> FirefoxB\\n}\\n';
    const compile = await rpc({type:'compile',source,address:'firefox.laml',
                               presentation:'Editor'});
    const svg = await rpc({type:'render_export',source,address:'firefox.laml',
                           format:'Svg'});
    // Reproduce the old policy, then prove restoring the header policy fixes it.
    const meta = document.createElement('meta');
    meta.name = 'referrer'; meta.content = 'no-referrer';
    document.head.appendChild(meta);
    const old = await rpc({type:'close_document'});
    meta.content = 'same-origin';
    const restored = await rpc({type:'close_document'});
    done({compile,svg,old:old.status,restored:restored.status});
})().catch(e => done({error:String(e)}));
''', 'args': [],
                })
                assert result['compile']['status'] == 200, result
                assert 'Ok' in result['compile']['body']['result']['Ok']['data'], result
                assert result['svg']['status'] == 200, result
                assert 'Ok' in result['svg']['body']['result'], result
                assert result['old'] == 403, result
                assert result['restored'] == 200, result
                with request(origin) as response:
                    assert response.headers['Referrer-Policy'] == 'same-origin'
                for rejected in ['null', 'https://unrelated.invalid']:
                    with request(origin + '/api/editor', {}, headers={
                        'Origin': rejected, 'Content-Type': 'application/json',
                        'X-LAML-Protocol': '1',
                    }) as response:
                        assert response.status == 403
                print('Firefox: compile/SVG passed; old-policy 403 reproduced; '
                      'restored policy passed; null/foreign origins rejected.')
            finally:
                if session:
                    request(driver_url + session, method='DELETE').close()
                for process in reversed(processes):
                    process.terminate()
                    try:
                        process.wait(timeout=10)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        process.wait()


if __name__ == '__main__':
    main()
