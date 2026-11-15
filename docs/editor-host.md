# Run the editor in a browser

`laml-ui serve` hosts the same Rust/egui application as the desktop editor.
Source editing, diagram gestures, properties, tabs and undo/redo are shared.
By default, documents live in the visitor's browser. The host only compiles and
renders complete requests in RAM. An explicit `--workspace` selects the separate
file-based workspace mode.

## Deploy a release with Docker Compose

Download the release's `laml-server-vVERSION-linux-amd64.tar.gz` and `SHA256SUMS`,
verify the archive, and extract it on a Linux x86-64 server. The archive contains
both native binaries, matching browser assets, `Dockerfile` and `compose.yaml`.
From its extracted directory:

```sh
docker compose up -d --build
```

Open port 7878 on the server. The image is assembled from the release's binaries;
no Rust compilation or source checkout runs on the server. Port mapping and
`--max-users` are configured directly in `compose.yaml`. Browser-local storage is
the default; no data volume is needed. For a public deployment, use your existing
HTTPS reverse proxy. See [the server bundle guide](../deploy/server/README.md) for
verification, configuration and updates.

A `vVERSION` Git tag builds CLI and editor archives for Linux x86-64, macOS
Intel/ARM and Windows x86-64, plus browser assets, this Linux amd64 server bundle,
a source archive and SHA-256 checksums. All artifacts come from that exact tag.

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

## Start: browser storage (default)

```sh
./target/release/laml-ui serve --assets "$PWD/target/editor-web" --max-users 10
```

From the repository root, `--assets` defaults to `target/editor-web`, so
`./target/release/laml-ui serve` is sufficient after building.

The **Browser library** button lists diagrams stored in IndexedDB for this browser
profile and site origin. **Open / Import .laml** reads local markup files; every
source or accepted visual edit is saved automatically, including incomplete and
invalid text. The footer distinguishes pending writes, acknowledged local saves
and storage failures. A storage failure warns the user to download their work;
closing a tab is blocked until its browser writes are acknowledged.

The library offers Open, Rename, Download and Move to trash. Trash entries can be
restored or permanently deleted. Empty trash and Delete everything require
confirmation. Clearing the library leaves no open document or automatic replacement.
Reload restores this tab's open documents. With no open documents, the editor
shows Create new file and Open from library instead of creating a placeholder.
The New file dialog requires a name, prefilled with an unused default; a newly
created file is saved in the library immediately, before its first edit.
Destructive library actions use a separate modal confirmation. Libraries are shared across browser
tabs; if two tabs edit the same document concurrently, the other version is
preserved as a separate library entry instead of silently overwritten.

**Download LAML** (Ctrl/Cmd+S) downloads the editable source. **Export** downloads
SVG, PNG or PDF. Downloads do not write host files. Sources must be self-contained:
inline styles, bundled fonts and bundled icons are supported; imported stylesheets
and local font files are rejected before compilation. No external assets can be
uploaded or resolved from the host filesystem.

Every compile, gesture and export request supplies its source. The host retains
no document session or compiler cache between requests and creates no user-data
files, temporary staging files or recovery snapshots. Native editing and workspace
mode keep their existing file/recovery adapters. Browser mode has no recovery
chooser because the library continuously stores edits locally.

**Clearing browser/site data deletes the library.** A different browser, device,
profile, host name or port has a different library. Keep `.laml` downloads as
backups; storage availability and quota belong to the browser. If the connection
is lost while the editor is open, source editing and browser storage remain
available; compilation, exports and visual editing need a backend connection.
Loading the application from scratch still requires the host.

### Concurrent browser sessions

`--max-users N` accepts a positive integer, default **10**. All tabs of the same
browser profile and origin share one place, regardless of the number of documents.
A place expires after **10 minutes without a heartbeat or request**; in-flight
requests retain their place until completion even if the client disconnects.
Returning sessions reacquire a place. A full server displays:

> The server is full. All available places are occupied. Local editing remains
> available; the connection will retry automatically.

Admission is retried automatically. This limits browser sessions, not diagram
size, request duration or the number of concurrent requests from one session.
There are no configured size/time/resource caps in browser mode. The existing
`--limits` and `--parallelism` switches belong to workspace mode; they are rejected
without `--workspace` rather than silently changing browser-mode behavior.

## Start: server workspace (explicit)

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

## Optional workspace deployment policies

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

## Workspace saves and recovery

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
explicitly. Workspace recovery is not written to browser storage. Navigation
warns about dirty buffers. Host backups remain useful for long-term retention.

## Local archive

```sh
./scripts/package-editor-host.sh
```

This creates a native host + matching browser assets, example and stylesheet,
guides and license notices, alongside a SHA-256 file. Existing outputs are never
replaced. Extract and start the default browser-storage mode from the archive directory:

```sh
./laml-ui serve --assets "$PWD/web"
```

Add `--workspace "$PWD/examples" --state-dir "$HOME/.local/state/laml-editor-host/state"`
to use server-side files instead.

Rust and Node are build/test tools, not runtime requirements for the archive.

## Targeted browser-storage acceptance

After building matching native/WASM debug artifacts and generating web assets:

```sh
node scripts/test-editor-browser-storage.mjs
```

This focused check covers browser writes/reload, invalid source, offline editing,
LAML downloads, trash/clear, shared-tab admission, conflicting tab writes and a
simulated storage failure. It uses an isolated browser profile and host directory.
`LAML_TEST_PORT` overrides its default local test port 17883.

## Workspace validation

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
