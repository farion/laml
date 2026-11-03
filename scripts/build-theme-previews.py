#!/usr/bin/env python3
"""Build transparent modern/light and modern-dark PNGs for every example.

Usage: python3 scripts/build-theme-previews.py [path/to/laml]
Temporary sources sit beside the originals so imports and fonts stay relative.
The original sources are never changed.
"""

import colorsys
from pathlib import Path
import re
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
EXAMPLES = ROOT / "examples"
IMAGES = ROOT / "docs" / "images"
SETTINGS = re.compile(r"(?m)^settings\s*\{")
PROPERTY = re.compile(r"(?m)^\s*(theme|background)\s*:[^\n]*\n?")
STYLE_RULE = re.compile(r"(?m)^([ \t]+)([^{}\n]+)\{([^{}]*)\}")
FILL = re.compile(r"(?m)^\s*fill\s*:\s*([^\n]+)")
HEX_PAINT = re.compile(r'(?m)^(\s*(?:color|border-color)\s*:\s*")(#[0-9a-fA-F]{6})')
STROKE = re.compile(r'(?m)^(\s*color\s*:\s*")(#[0-9a-fA-F]{6})')


def rgb_of(value: str) -> list[int] | None:
    value = value.strip().strip('"')
    if re.fullmatch(r"#[0-9a-fA-F]{6}", value):
        return [int(value[i:i + 2], 16) for i in (1, 3, 5)]
    if match := re.fullmatch(r"rgb\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*\)", value):
        return [int(channel) for channel in match.groups()]
    return None


def brighten(paint: re.Match[str]) -> str:
    rgb = rgb_of(paint[2])
    if sum(rgb) / 3 >= 145:
        return paint[0]
    lighter = "#" + "".join(f"{round(0.45 * c + 0.55 * 255):02x}" for c in rgb)
    return paint[1] + lighter


def adapt_dark_paints(source: str) -> str:
    # Explicit LSS paints take precedence over theme defaults. Adapt them in
    # temporary dark sources instead of publishing pale boxes on a dark page.
    def recolor(match: re.Match[str]) -> str:
        indent, selector, declarations = match.groups()
        kind = selector.strip()
        fill = FILL.search(declarations)
        if kind.startswith(("node", "note", "fragment", "continuation")) and fill:
            rgb = rgb_of(fill[1])
            if rgb and sum(rgb) / 3 > 180:
                hue, _, saturation = colorsys.rgb_to_hls(*(c / 255 for c in rgb))
                shade = colorsys.hls_to_rgb(hue, 0.17, min(saturation, 0.42))
                dark = "#" + "".join(f"{round(c * 255):02x}" for c in shade)
                declarations = declarations.replace(fill[1], f'"{dark}"', 1)
                declarations = HEX_PAINT.sub(brighten, declarations)
        if kind.startswith("relation") or " lifeline" in kind:
            declarations = STROKE.sub(brighten, declarations)
        return f"{indent}{selector}{{{declarations}}}"

    return STYLE_RULE.sub(recolor, source)


def variant(source: str, theme: str) -> str:
    overrides = f"    theme: {theme}\n    background: transparent\n"
    match = SETTINGS.search(source)
    if match is None:
        result = "settings {\n" + overrides + "}\n" + source
    else:
        end = source.find("}", match.end())
        if end < 0:
            raise ValueError("unterminated settings block")
        body = PROPERTY.sub("", source[match.end():end])
        result = source[:match.end()] + "\n" + overrides + body.lstrip("\n") + source[end:]
    return adapt_dark_paints(result) if theme == "modern-dark" else result


def main() -> int:
    compiler = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else ROOT / "target/release/laml"
    if not compiler.is_file():
        print(f"compiler not found: {compiler}", file=sys.stderr)
        return 1
    IMAGES.mkdir(parents=True, exist_ok=True)
    failures = []
    count = 0
    for source in sorted(EXAMPLES.glob("*.laml")):
        original = source.read_text(encoding="utf-8")
        for theme in ("modern", "modern-dark"):
            destination = IMAGES / f"{source.stem}-{theme}.png"
            with tempfile.NamedTemporaryFile(
                mode="w", encoding="utf-8", suffix=".laml", prefix=".theme-preview-",
                dir=EXAMPLES, delete=False,
            ) as temporary:
                temporary.write(variant(original, theme))
                temporary_path = Path(temporary.name)
            try:
                result = subprocess.run(
                    [str(compiler), "build", str(temporary_path), str(destination)],
                    capture_output=True, text=True, check=False,
                )
            finally:
                temporary_path.unlink(missing_ok=True)
            if result.returncode:
                failures.append(f"{source.name} ({theme}): {result.stderr.strip()}")
            else:
                count += 1
    print(f"Built {count} transparent previews in {IMAGES}")
    for failure in failures:
        print(failure, file=sys.stderr)
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
