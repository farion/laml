# Run the editor in a browser

`laml-ui serve` hosts the same Rust/egui application as the desktop editor.
Source editing, diagram gestures, properties, tabs and undo/redo are shared.
Files, compilation, dependencies, exports and recovery snapshots stay on the host.

## Build

From the repository root:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129 --locked --root target/web-tools
cargo build --locked --release -p laml-ui
./scripts/build-editor-web.sh
```

The executable is `target/release/laml-ui` (`.exe` on Windows); browser assets are
in `target/editor-web`. The shell scripts can run in Git Bash on Windows.

## Start

```sh
./target/release/laml-ui serve \
  --workspace "$PWD/examples" \
  --assets "$PWD/target/editor-web" \
  --state-dir "$HOME/.local/state/laml-editor-host/state" \
  --bind 127.0.0.1:7878
```

Open the address printed by the host. The editor opens directly. Use **Open** and
enter `architecture.laml` for the supplied example. All paths are relative to the
configured workspace; nested addresses use forward slashes. **Save as** and
**Export** write files there, on the host rather than the visitor's computer.

`--bind IP:PORT` selects a listening interface and port. It is a normal network
parameter, independent of any VPN or network provider. A non-loopback bind works
with HTTP. `--origin URL` optionally overrides the advertised origin, for example
when a reverse proxy provides HTTPS. The LAML process itself serves HTTP; a TLS
reverse proxy supplies HTTPS when desired.

**Anyone who can reach the listener can read and modify allowed workspace files.**
Choose the interface, firewall, tunnel or reverse proxy appropriate for the people
who should have access. Access control and transport encryption belong to that
deployment. Keep state outside the workspace and public assets.

Source addresses end in `.laml`; stylesheet/font dependencies must resolve inside
the workspace. Scope checks, typed protocol validation, checked overwrites,
revision guards and source/dependency protection always preserve editing safety.

## Optional deployment policies

All options below are **off by default** and independent:

| Option | Enables |
| --- | --- |
| `--limits` | The bounded resource/size profile described below. |
| `--parallelism N` | At most N active compiler requests across clients. Omit for no configured concurrency cap. |
| `--http-checks` | Exact Host/Origin checks and restrictive HTTP security headers. |
| `--require-https` | Requires an HTTPS `--origin` for deployment behind a TLS reverse proxy. It does not activate TLS in the process. |

Example with explicitly selected policies:

```sh
./target/release/laml-ui serve \
  --workspace /srv/diagrams --assets /opt/laml-editor/web \
  --state-dir /srv/laml-state --bind 127.0.0.1:7878 \
  --origin https://diagrams.example.org \
  --limits --parallelism 2 --http-checks --require-https
```

With `--limits`, the profile applies 16 MiB request / 64 MiB response sizes,
4 MiB source, 32 MiB per dependency / 64 MiB staged total, 128 dependencies,
32 nested import levels, 512 nodes / 2,048 relations, bounded retained export and
recovery storage, and 40-second compiler deadlines. Unix CPU/address-space limits
or Windows job objects bound compiler resources (1,536 MiB memory). Browser request
timeouts and matching protocol size checks are enabled with this same option.
`--limits` does not set `--parallelism`.

Without these options, these application-level restrictions and deadlines are
not enabled. Machine capacity, filesystem limits and representable wire-format
lengths still apply. Requests sharing mutable compiler state are serialized for
correctness; separate browser clients can run independently.

## Saves and recovery

A save acknowledges the submitted snapshot; typing during the write remains
dirty. Existing targets need explicit confirmation and are checked again before
writing. A clean source reloads after an external change; a dirty buffer is kept
for an explicit conflict decision. Invalid text retains the last valid image and
locks visual mutation until compilation succeeds.

Exports use one-use host-held tickets. Commit rechecks dependencies and output
contents. A disconnected request does not imply an accepted write was cancelled.

Recovery files live in `STATE_DIR/recovery`, using the host's normal file leases
and protocol document identities. Connected clients use the same workspace and
recovery directory. Background tabs and browser inactivity keep their live leases;
there is no idle expiry. Closing or reloading a page sends a departure notification
that releases its worker after accepted requests finish. Its abandoned snapshots
then become available. If the browser exits without delivering that notification,
the leases remain until the host stops; snapshots are retained on disk.
**Restore** changes an editor buffer, never the original source file; use **Save**
explicitly. Application recovery is not written to browser storage. Navigation
warns about dirty buffers. Host backups remain useful for long-term retention.

## Local archive

```sh
./scripts/package-editor-host.sh
```

This creates a native host + matching browser assets, example and stylesheet,
guides and license notices, alongside a SHA-256 file. Existing outputs are never
replaced. Extract and start from the archive directory:

```sh
./laml-ui serve --workspace "$PWD/examples" --assets "$PWD/web" \
  --state-dir "$HOME/.local/state/laml-editor-host/state"
```

Rust and Node are build/test tools, not runtime requirements for the archive.

## Validation

```sh
cargo test --locked --workspace --release
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo clippy --locked -p laml-ui --lib --target wasm32-unknown-unknown -- -D warnings
node scripts/test-editor-browser.mjs
node scripts/test-editor-browser.mjs --limits
python3 scripts/test-editor-package.py target/laml-editor-host-linux-x86_64.tar.gz
```

Browser tests use Node's WebSocket API and Chrome (`google-chrome`, or set
`CHROME_BIN`). They run both default and explicitly bounded configurations, source
and diagram editing, shared undo/redo, inspector edits, SVG/PNG/PDF exports and
reload/recovery, and a browser execution pause longer than twelve seconds while
another client checks recovery. The package test uses Python 3.12+; `--native-capture` additionally
captures the packaged desktop editor in a local desktop session.

Local native/browser acceptance is Linux/Chrome. Other platforms, browsers and a
second physical client have their own acceptance scope.
