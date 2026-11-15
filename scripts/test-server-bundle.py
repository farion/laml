#!/usr/bin/env python3
"""Build the released Docker context and smoke-test headless browser hosting."""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile
import time
import urllib.error
import urllib.request

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("bundle", type=Path, help="Extracted server bundle directory")
args = parser.parse_args()
bundle = args.bundle.resolve()
for name in ["bin/laml", "bin/laml-ui", "web/laml_ui.js", "web/laml_ui_bg.wasm",
             "Dockerfile", "compose.yaml", "SOURCE.txt", "licenses/EDITOR-GPL-3.0.txt"]:
    assert (bundle / name).is_file(), name
subprocess.run(["docker", "compose", "config", "--quiet"], cwd=bundle, check=True)
with tempfile.TemporaryDirectory(prefix="laml-container-test-") as temp:
    tag = "laml-server-smoke:" + Path(temp).name.lower()
    container = None
    try:
        subprocess.run(["docker", "build", "--tag", tag, str(bundle)], check=True)
        container = subprocess.check_output([
            "docker", "run", "--detach", "--publish", "127.0.0.1::7878", tag
        ], text=True).strip()
        port = subprocess.check_output(["docker", "port", container, "7878/tcp"], text=True).strip()
        origin = "http://" + port
        opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
        for attempt in range(100):
            try:
                with opener.open(origin + "/api/options", timeout=2) as response:
                    options = json.load(response)
                break
            except (OSError, urllib.error.URLError):
                if attempt == 99:
                    raise
                time.sleep(0.1)
        assert options["browser"] is True and options["max_users"] == 10, options
        for path, prefix in [("/", b"<!doctype html"), ("/pkg/laml_ui_bg.wasm", b"\0asm")]:
            with opener.open(origin + path) as response:
                assert response.read(20).lower().startswith(prefix), path
        source = "semantics {\n ServerSmoke\n}\n"
        for number, operation in enumerate([
            {"type": "compile", "source": source, "address": "test.laml", "presentation": "Editor"},
            {"type": "render_export", "source": source, "address": "test.laml", "format": "Svg"},
            {"type": "render_export", "source": source, "address": "test.laml", "format": "Png"},
            {"type": "render_export", "source": source, "address": "test.laml", "format": "Pdf"},
        ], 1):
            body = json.dumps({"stamp": {"version": 1, "session": 1, "document": 1,
                                        "request": number, "revision": 0}, "operation": operation}).encode()
            request = urllib.request.Request(origin + "/api/editor", body, headers={
                "Content-Type": "application/json", "X-LAML-Protocol": "1",
                "X-LAML-Session": "1234567890abcdef1234567890abcdef",
            })
            with opener.open(request, timeout=60) as response:
                result = json.load(response)["result"]
            assert "Ok" in result, result
            if operation["type"] == "compile":
                assert "<svg" in result["Ok"]["data"]["Ok"]["svg"], result
            else:
                assert result["Ok"]["data"], result
        changes = subprocess.check_output(["docker", "diff", container], text=True)
        assert not changes.strip(), "Host unexpectedly wrote container files: " + changes
        print("Server bundle passed: Docker build, headless startup, browser mode, WASM, compile, SVG/PNG/PDF, no host writes.")
    finally:
        if container:
            subprocess.run(["docker", "logs", "--tail", "10", container], check=False)
            subprocess.run(["docker", "rm", "--force", container], stdout=subprocess.DEVNULL, check=False)
        subprocess.run(["docker", "image", "rm", tag], stdout=subprocess.DEVNULL, check=False)
