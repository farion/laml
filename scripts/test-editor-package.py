#!/usr/bin/env python3
"""Check a local host archive with no repository runtime resources or login."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tarfile
import tempfile
import time
import urllib.request

repo=Path(__file__).resolve().parent.parent
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument("archive",type=Path)
parser.add_argument("--native-capture",action="store_true")
args=parser.parse_args()
archive=args.archive.resolve()
checksum=archive.with_name(archive.name+".sha256")
with archive.open("rb") as file:
    assert hashlib.file_digest(file,"sha256").hexdigest()==checksum.read_text().split()[0]
root=Path(tempfile.mkdtemp(prefix="host-package-",dir=repo/"target"))
with tarfile.open(archive) as tar:
    tar.extractall(root/"unpacked",filter="data")
bundle,=(root/"unpacked").iterdir()
executable=bundle/("laml-ui.exe" if os.name=="nt" else "laml-ui")
env=dict(os.environ,LAML_NO_UPDATE_CHECK="1")
for key in ("DISPLAY","WAYLAND_DISPLAY","WAYLAND_SOCKET"):
    env.pop(key,None)
with (root/"host.log").open("w+") as log:
    host=subprocess.Popen([str(executable),"serve","--workspace",str(bundle/"examples"),"--assets",str(bundle/"web"),"--state-dir",str(root/"state"),"--bind","127.0.0.1:0"],cwd=tempfile.gettempdir(),env=env,stdout=log,stderr=log)
    try:
        origin=None
        for _ in range(200):
            log.flush();log.seek(0)
            match=re.search(r"LAML host: (http://[^\s]+)",log.read())
            if match:
                origin=match.group(1);break
            assert host.poll() is None,"Packaged host stopped during startup"
            time.sleep(.05)
        assert origin,"Host did not acknowledge startup"
        opener=urllib.request.build_opener(urllib.request.ProxyHandler({}))
        with opener.open(origin+"/api/options") as response:
            assert response.headers.get("Set-Cookie") is None
            assert json.load(response)=={"limits":False,"parallelism":None,"http_checks":False,"require_https":False}
        with opener.open(origin+"/pkg/laml_ui_bg.wasm") as response:
            assert response.read(4)==b"\0asm"
        source=(bundle/"examples/architecture.laml").read_text()
        def rpc(number,operation):
            body=json.dumps({"stamp":{"version":1,"session":321,"document":1,"request":number,"revision":0},"operation":operation}).encode()
            request=urllib.request.Request(origin+"/api/editor",body,headers={"Content-Type":"application/json","X-LAML-Protocol":"1"},method="POST")
            with opener.open(request,timeout=60) as response:
                assert response.headers.get("Set-Cookie") is None
                result=json.load(response)["result"]
                assert "Err" not in result,"Host rejected packaged operation"
                return result["Ok"]
        model=rpc(1,{"type":"compile","source":source,"address":"architecture.laml","presentation":"Editor"})
        assert "<svg" in model["data"]["Ok"]["svg"]
        assert str(root) not in json.dumps(model)
        ticket=rpc(2,{"type":"begin_export","address":"packaged.svg","source_address":"architecture.laml","source":source,"format":"Svg"})["data"]["ticket"]
        rpc(3,{"type":"commit_export","ticket":ticket})
        assert "<svg" in (bundle/"examples/packaged.svg").read_text()
        print("Extracted archive: startup, policy defaults, WASM, real example/import and checked export passed.")
    finally:
        host.terminate()
        try:host.wait(timeout=5)
        except subprocess.TimeoutExpired:host.kill();host.wait()
before=(archive.stat().st_mtime_ns,checksum.read_bytes())
probe=subprocess.run(["sh",str(repo/"scripts/package-editor-host.sh"),str(archive)],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
assert probe.returncode!=0 and before==(archive.stat().st_mtime_ns,checksum.read_bytes())
print("Archive SHA-256 and no-clobber passed.")
if args.native_capture:
    env=dict(os.environ,LAML_UI_STATE_DIR=str(root/"native-state"),LAML_NO_UPDATE_CHECK="1")
    for key in ("WAYLAND_DISPLAY","WAYLAND_SOCKET"):env.pop(key,None)
    with (root/"native.log").open("w") as log:
        subprocess.run([str(executable),"--capture",str(root/"editor.png"),str(bundle/"examples/architecture.laml")],env=env,stdout=log,stderr=log,timeout=60,check=True)
    assert (root/"editor.png").read_bytes().startswith(b"\x89PNG")
    print("Packaged native capture passed.")
print("Evidence:",root.relative_to(repo))
