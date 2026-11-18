#!/usr/bin/env python3
"""Run only the copied executables, with no repository assets beside them."""
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

root = Path(sys.argv[1]).resolve()
suffix = ".exe" if os.name == "nt" else ""
with tempfile.TemporaryDirectory(prefix="laml-standalone-") as temporary:
    folder = Path(temporary)
    cli = folder / ("laml" + suffix)
    editor = folder / ("laml-ui" + suffix)
    shutil.copy2(root / cli.name, cli)
    shutil.copy2(root / editor.name, editor)
    env = dict(os.environ, LAML_NO_UPDATE_CHECK="1")

    def run(*args):
        result = subprocess.run(args, cwd=folder, env=env, check=True,
                                text=True, capture_output=True, timeout=60)
        return result.stdout

    assert "laml " in run(str(cli), "--version")
    assert "laml-ui " in run(str(editor), "--version")
    assert "clock" in run(str(cli), "icons", "clock").splitlines()
    assert "Phosphor" in run(str(cli), "--licenses")
    assert "DejaVu" in run(str(editor), "--licenses")
    source = folder / "standalone.laml"
    source.write_text('semantics {\n A(label="Hello")\n B(label="World")\n A -> B\n}\n', encoding="utf-8")
    run(str(cli), "check", str(source))
    for extension in ("svg", "png", "pdf"):
        output = folder / ("diagram." + extension)
        run(str(cli), "build", str(source), str(output))
        assert output.stat().st_size > 100, extension
    print("Standalone CLI/editor start; bundled icons and SVG/PNG/PDF rendering passed.")
