#!/usr/bin/env python3
"""Generate editor relation swatches with the LAML compiler's SVG renderer."""
from pathlib import Path
import re
import subprocess
import tempfile
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "crates/laml-ui/assets/relation-types"
NS = "{http://www.w3.org/2000/svg}"
ET.register_namespace("", NS[1:-1])
operators = re.search(
    r"pub const OPERATORS:.*?= &\[(.*?)\];",
    (ROOT / "crates/laml-ui/src/relation_edit.rs").read_text(), re.S,
).group(1)
OUT.mkdir(parents=True, exist_ok=True)
with tempfile.TemporaryDirectory(prefix="relation-previews-") as temp:
    source = Path(temp) / "preview.laml"
    image = Path(temp) / "preview.svg"
    for index, operator in enumerate(re.findall(r'"([^"]+)"', operators)):
        reverse = {"<-": "->", "<--": "-->", "<..": "..>", "<|--": "--|>", "<|..": "..|>"}
        relation = f"B {reverse[operator]} A" if operator in reverse else f"A {operator} B"
        source.write_text(
            "settings {\n theme: modern-dark\n background: transparent\n}\n"
            f"semantics {{\n A\n B\n {relation}\n}}\n"
            "layout {\n A < B : align=center; gap=120\n}\n"
        )
        subprocess.run([ROOT / "target/release/laml", "build", source, image], check=True)
        root = ET.parse(image).getroot()
        nodes = root.findall(f"{NS}g/{NS}rect")
        a, b = nodes[:2]
        x = float(a.get("x")) + float(a.get("width")) - 5
        y = float(a.get("y")) + float(a.get("height")) / 2 - 14
        width = float(b.get("x")) - x + 5
        swatch = ET.Element(f"{NS}svg", {
            "width": str(width), "height": "28",
            "viewBox": f"{x} {y} {width} 28",
        })
        defs = ET.SubElement(swatch, f"{NS}defs")
        for definition in root.find(f"{NS}defs"):
            if definition.tag == f"{NS}marker":
                defs.append(definition)
        for element in root:
            if element.tag not in [f"{NS}g", f"{NS}defs", f"{NS}style"]:
                swatch.append(element)
        ET.ElementTree(swatch).write(OUT / f"{index:02}.svg", encoding="unicode")
