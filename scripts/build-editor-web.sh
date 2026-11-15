#!/usr/bin/env sh
# Build the shared Rust/egui shell for browsers. No separate editor implementation.
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
if command -v wasm-bindgen >/dev/null 2>&1; then
  BINDGEN=wasm-bindgen
elif [ -x target/web-tools/bin/wasm-bindgen ]; then
  BINDGEN=target/web-tools/bin/wasm-bindgen
else
  printf '%s\n' 'Install wasm-bindgen-cli 0.2.129 and the wasm32-unknown-unknown Rust target first.' >&2
  exit 1
fi
case "$($BINDGEN --version)" in
  'wasm-bindgen 0.2.129') ;;
  *) printf '%s\n' 'This build requires wasm-bindgen-cli 0.2.129.' >&2; exit 1 ;;
esac
cargo build --locked --release --lib -p laml-ui --target wasm32-unknown-unknown
mkdir -p target/editor-web
"$BINDGEN" --target web --out-dir target/editor-web target/wasm32-unknown-unknown/release/laml_ui.wasm
printf '%s\n' 'Web assets: target/editor-web' 'Host: target/release/laml-ui serve --help'
