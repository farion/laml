#!/usr/bin/env sh
# Assemble prebuilt Linux amd64 artifacts; never compile Rust on the server.
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
version="$(sed -n 's/^version = "\([^"]*\)"$/\1/p' Cargo.toml | head -n 1)"
name="laml-server-v${version}-linux-amd64"
native="${LAML_NATIVE_DIR:-target/release}"
out="${1:-target/$name.tar.gz}"
if [ "$#" -gt 1 ] || [ -e "$out" ] || [ -L "$out" ]; then
  printf '%s\n' 'Choose one new archive path; existing outputs are never replaced.' >&2
  exit 1
fi
[ "$(uname -s)" = Linux ] && [ "$(uname -m)" = x86_64 ] || {
  printf '%s\n' 'Package on Linux x86-64 using matching release artifacts.' >&2; exit 1;
}
for file in "$native/laml" "$native/laml-ui" target/editor-web/laml_ui.js target/editor-web/laml_ui_bg.wasm; do
  test -s "$file" || { printf 'Build first; missing %s\n' "$file" >&2; exit 1; }
done
[ "$("$native/laml" --version)" = "laml $version" ] || {
  printf '%s\n' 'Compiler binary does not match Cargo.toml.' >&2; exit 1;
}
mkdir -p target "$(dirname -- "$out")"
stage="$(mktemp -d target/server-package.XXXXXX)"
trap 'rm -rf "$stage"' EXIT HUP INT TERM
folder="$stage/$name"
mkdir -p "$folder/bin" "$folder/web" "$folder/licenses"
cp "$native/laml" "$native/laml-ui" "$folder/bin/"
cp -R target/editor-web/. "$folder/web/"
cp deploy/server/Dockerfile deploy/server/.dockerignore deploy/server/README.md "$folder/"
sed "s/image: laml-editor:.*/image: laml-editor:$version/" deploy/server/compose.yaml > "$folder/compose.yaml"
cp LICENSE "$folder/licenses/COMPILER-MIT.txt"
cp crates/laml-ui/LICENSE "$folder/licenses/EDITOR-GPL-3.0.txt"
cp assets/INTER-LICENSE.txt assets/GEIST-LICENSE.txt assets/GEIST-SOURCE.txt assets/DEJAVU-LICENSE.txt "$folder/licenses/"
cp assets/phosphor/LICENSE "$folder/licenses/PHOSPHOR-MIT.txt"
cat > "$folder/SOURCE.txt" <<EOF
LAML v$version
Source tag: https://github.com/farion/laml/tree/v$version
Corresponding source: laml-v$version-source.tar.gz in the same GitHub release.
Compiler: MIT. Editor: GPL-3.0-or-later.
Build native binaries: cargo build --locked --release --workspace
Build matching browser assets: scripts/build-editor-web.sh
Build and packaging instructions: docs/editor-host.md
EOF
tar -czf "$stage/archive.tar.gz" -C "$stage" "$name"
ln "$stage/archive.tar.gz" "$out"
printf 'Server package: %s\n' "$out"
