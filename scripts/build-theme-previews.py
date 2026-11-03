#!/usr/bin/env python3
"""Build transparent modern and modern-dark PNGs for every .laml example.

Usage: python3 scripts/build-theme-previews.py [path/to/laml]
Temporary source files are created beside the original, preserving relative
style imports and fonts. The original sources are never changed.
"""

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


def light_fill(value: str) -> bool:
    value = value.strip().strip('"')
    if re.fullmatch(r"#[0-9a-fA-F]{6}", value):
        rgb = [int(value[i:i + 2], 16) for i in (1, 3, 5)]
    elif match := re.fullmatch(r"rgb\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*\)", value):
        rgb = [int(channel) for channel in match.groups()]
    else:
        return False
    return sum(rgb) / 3 > 180


def contrast_for_dark_theme(source: str) -> str:
    # Explicit light LSS fills remain light in dark themes. Give those shapes
    # their matching dark text; otherwise dark-theme default text disappears.
    def readable(match: re.Match[str]) -> str:
        indent, selector, declarations = match.groups()
        if not selector.strip().startswith(("node", "note", "fragment", "continuation")):
            return match.group()
        fill = FILL.search(declarations)
        if not fill or not light_fill(fill[1]) or re.search(r"(?m)^\s*color\s*:", declarations):
            return match.group()
        return f"{indent}{selector}{{{declarations}{indent}    color: \"#243040\"\n{indent}}}"

    source = STYLE_RULE.sub(readable, source)

    def readable_strokes(match: re.Match[str]) -> str:
        indent, selector, declarations = match.groups()
        if not (selector.strip().startswith("relation") or " lifeline" in selector):
            return match.group()

        def lighten(color: re.Match[str]) -> str:
            rgb = [int(color[2][i:i + 2], 16) for i in (1, 3, 5)]
            if sum(rgb) / 3 >= 145:
                return color[0]
            lighter = "#" + "".join(f"{round(0.45 * c + 0.55 * 255):02x}" for c in rgb)
            return color[1] + lighter

        declarations = re.sub(
            r'(?m)^(\s*color\s*:\s*")(#[0-9a-fA-F]{6})',
            lighten,
            declarations,
        )
        return f"{indent}{selector}{{{declarations}}}"

    return STYLE_RULE.sub(readable_strokes, source)


def variant(source: str, theme: str) -> str:
    overrides = f"    theme: {theme}\n    background: transparent\n"
    match = SETTINGS.search(source)
    if match is None:
        result = "settings {\n" + overrides + "}\n" + source
        return contrast_for_dark_theme(result) if theme == "modern-dark" else result
    end = source.find("}", match.end())
    if end < 0:
        raise ValueError("unterminated settings block")
    body = PROPERTY.sub("", source[match.end():end])
    result = source[:match.end()] + "\n" + overrides + body.lstrip("\n") + source[end:]
    return contrast_for_dark_theme(result) if theme == "modern-dark" else result


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
