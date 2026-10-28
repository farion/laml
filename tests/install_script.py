#!/usr/bin/env python3
"""Exercise the POSIX installer against a local mock release server."""
import hashlib
import http.server
import io
import os
import platform
from pathlib import Path
import subprocess
import tarfile
import tempfile
import threading

ROOT = Path(__file__).resolve().parents[1]
TARGETS = {
    ("Linux", "x86_64"): "x86_64-unknown-linux-gnu",
    ("Darwin", "x86_64"): "x86_64-apple-darwin",
    ("Darwin", "arm64"): "aarch64-apple-darwin",
}
NAME = "laml-v1.1.0-" + TARGETS[(platform.system(), platform.machine())]
BINARY = b"#!/bin/sh\necho mock-laml\n"


def archive_bytes():
    stream = io.BytesIO()
    with tarfile.open(fileobj=stream, mode="w:gz") as archive:
        info = tarfile.TarInfo(NAME + "/laml")
        info.size = len(BINARY)
        info.mode = 0o755
        archive.addfile(info, io.BytesIO(BINARY))
    return stream.getvalue()


class ReleaseHandler(http.server.BaseHTTPRequestHandler):
    data = archive_bytes()
    invalid = False

    def do_GET(self):
        if self.path == "/releases/latest":
            self.send_response(302)
            self.send_header("Location", "/releases/tag/v1.1.0")
            self.end_headers()
            return
        if self.path == "/releases/tag/v1.1.0":
            body = b"release"
        elif self.path == f"/releases/download/v1.1.0/{NAME}.tar.gz":
            body = self.data
        elif self.path == "/releases/download/v1.1.0/SHA256SUMS":
            checksum = hashlib.sha256(self.data).hexdigest()
            if self.invalid:
                checksum = "0" * 64
            body = f"{checksum}  {NAME}.tar.gz\n".encode()
        else:
            self.send_error(404)
            return
        self.send_response(200)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, format, *args):
        pass


with tempfile.TemporaryDirectory(prefix="laml-installer-test-") as temp:
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), ReleaseHandler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        script = (ROOT / "scripts/install.sh").read_text().replace(
            "repo=https://github.com/farion/laml",
            f"repo=http://127.0.0.1:{server.server_port}",
        )
        assert "repo=http://127.0.0.1:" in script
        install_dir = Path(temp) / "bin"
        env = {**os.environ, "LAML_INSTALL_DIR": str(install_dir)}
        success = subprocess.run(["sh"], input=script, text=True, env=env, capture_output=True)
        assert success.returncode == 0, success.stderr
        assert (install_dir / "laml").read_bytes() == BINARY
        ReleaseHandler.invalid = True
        failure = subprocess.run(["sh"], input=script, text=True, env=env, capture_output=True)
        assert failure.returncode != 0
        assert "checksum mismatch" in failure.stderr
        assert (install_dir / "laml").read_bytes() == BINARY
    finally:
        server.shutdown()
        thread.join(timeout=2)
print("POSIX install and checksum rejection passed")
