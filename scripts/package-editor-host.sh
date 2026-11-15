#!/usr/bin/env sh
# Package the built shared desktop/browser editor. Runtime state stays separate.
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
case "$(uname -s)" in
  Linux) platform=linux; executable=laml-ui ;;
  Darwin) platform=macos; executable=laml-ui ;;
  MINGW*|MSYS*) platform=windows; executable=laml-ui.exe ;;
  *) printf '%s\n' 'Unsupported packaging host' >&2; exit 1 ;;
esac
name="laml-editor-host-$platform-$(uname -m)"
out="${1:-target/$name.tar.gz}"
if [ "$#" -gt 1 ] || [ -e "$out" ] || [ -L "$out" ] || [ -e "$out.sha256" ] || [ -L "$out.sha256" ]; then
  printf '%s\n' 'Choose one new archive path; existing outputs are never replaced.' >&2
  exit 1
fi
for file in "target/release/$executable" target/editor-web/laml_ui.js target/editor-web/laml_ui_bg.wasm; do
  test -s "$file" || { printf 'Build first; missing %s\n' "$file" >&2; exit 1; }
done
mkdir -p target "$(dirname -- "$out")"
stage="$(mktemp -d target/editor-host-package.XXXXXX)"
trap 'rm -rf "$stage"' EXIT HUP INT TERM
folder="$stage/$name"
mkdir -p "$folder/web" "$folder/docs" "$folder/examples"
cp "target/release/$executable" "$folder/"
cp -R target/editor-web/. "$folder/web/"
cp docs/editor.md docs/editor-host.md docs/editor-installation.md "$folder/docs/"
cp examples/architecture.laml examples/architecture.lamlstyle "$folder/examples/"
cp LICENSE assets/INTER-LICENSE.txt assets/GEIST-LICENSE.txt assets/GEIST-SOURCE.txt "$folder/"
cp assets/phosphor/LICENSE "$folder/PHOSPHOR-LICENSE.txt"
printf '%s\n' '# LAML editor host' '' 'See docs/editor-host.md for startup and optional deployment policies.' 'The web/ directory contains the matching browser editor. Keep runtime state outside this folder.' > "$folder/README.md"
tar -czf "$stage/archive.tar.gz" -C "$stage" "$name"
if command -v sha256sum >/dev/null 2>&1; then
  digest="$(sha256sum "$stage/archive.tar.gz" | cut -d ' ' -f 1)"
else
  digest="$(shasum -a 256 "$stage/archive.tar.gz" | cut -d ' ' -f 1)"
fi
printf '%s  %s\n' "$digest" "$(basename -- "$out")" > "$stage/archive.sha256"
# Link only after complete construction. Existing files/symlinks cannot be replaced.
ln "$stage/archive.tar.gz" "$out"
ln "$stage/archive.sha256" "$out.sha256"
printf 'Archive: %s\nSHA-256: %s.sha256\n' "$out" "$out"
