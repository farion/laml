# Install the native LAML editor

The editor is a separate application named `laml-ui`. The compiler CLI remains
`laml`; both can be installed side by side. Editor release tags use `ui-vVERSION`
and archives start with `laml-ui-vVERSION`.

The same executable can host the shared editor in a browser with `laml-ui serve`.
See [browser build, local archive and host startup](editor-host.md) for its matching
WASM assets, authentication and server-side file workflow.

Choose the archive matching the operating system and CPU, and compare its SHA-256
with `SHA256SUMS` from that same release before extracting it. Keep the supplied
license notices with redistributed copies.

## Linux x86-64

Extract the `.tar.gz`, then start `./laml-ui` from its directory. A graphical X11 or
Wayland session and an OpenGL-capable driver are required. For a user-local command:

```sh
mkdir -p "$HOME/.local/bin"
install -m 755 laml-ui "$HOME/.local/bin/laml-ui"
laml-ui path/to/diagram.laml
```

File dialogs use the desktop portal. **Path…** and **Save path…** also accept an
explicit path when working in a minimal desktop environment.

## macOS Intel or Apple Silicon

Extract the archive for your CPU and move `LAML Editor.app` to your user
Applications folder. Open the application from Finder. Follow your organization's
software approval process for unsigned builds. The executable also accepts a file:

```sh
"LAML Editor.app/Contents/MacOS/laml-ui" path/to/diagram.laml
```

## Windows x86-64

Extract the ZIP into a user-writable application folder and double-click
`laml-ui.exe`. Open a source file using **Open…**, or pass a path when launching:

```powershell
.\laml-ui.exe C:\diagrams\architecture.laml
```

The release build uses the GUI subsystem so a console window is not needed.

## Files and recovery

Save `.laml` documents explicitly. Recovery snapshots are local, private files;
startup offers restoration or explicit discard after an interrupted session.
The original diagram on disk is never automatically overwritten by restoration.

Storage defaults:

- Linux: `$XDG_STATE_HOME/laml-ui/recovery`, otherwise
  `$HOME/.local/state/laml-ui/recovery`.
- macOS: `$HOME/Library/Application Support/LAML/recovery`.
- Windows: `%LOCALAPPDATA%\LAML\recovery`.

`LAML_UI_STATE_DIR` selects a different local state directory; snapshots use its
`recovery` subdirectory. The editor warns about snapshot/history size limits.

## Build and validate from source

```sh
cargo test --locked --workspace --all-targets
cargo build --locked --release -p laml
cargo build --locked --release -p laml-ui
./target/release/laml-ui examples/architecture.laml
```

For an automated native rendering capture of the editor's own window:

```sh
./target/release/laml-ui --capture target/editor.png examples/architecture.laml
```

The capture path must not already exist. It is a screenshot of the application,
not an SVG/PNG diagram export. Use **Export…** for compiler SVG/PNG/vector-PDF output.
