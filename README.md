# LAML

LAML is a diagram language and Rust compiler. A `.laml` source file produces one SVG, PNG, or PDF image. Relations describe the diagram; mandatory layout rules let the author control node positions without manually drawing them. Styling lives in LAML Style Sheets (LSS), either inline or in local imported files.

## Install the latest release

Linux and macOS (automatically selects x86-64 or Apple Silicon):

```sh
curl -fsSL https://raw.githubusercontent.com/farion/laml/main/scripts/install.sh | sh
```

Windows x86-64 (PowerShell):

```powershell
irm https://raw.githubusercontent.com/farion/laml/main/scripts/install.ps1 | iex
```

The installers fetch the latest **published** release over HTTPS, verify the archive against its `SHA256SUMS`, and install without administrator privileges. The shell installer uses `~/.local/bin` (override with `LAML_INSTALL_DIR`); the Windows installer uses `%LOCALAPPDATA%\Programs\laml`, adds it to the user PATH, and requires a new terminal. Checksums detect corruption; they are not digital signatures. Review any downloaded script before piping it to a shell if you need to audit it.

## Update behaviour

`laml --version` prints the installed version. `laml update` explicitly downloads and verifies the latest published release, then replaces the **running binary's installation**. On Windows the verified replacement is staged and applied after the process exits. A read-only or package-managed installation may require updating through its package manager instead.

Normal `laml check` and `laml build` invocations read a local update cache and never wait for the network. At most once every 24 hours a background process queries GitHub for the latest published version; a known newer version prints a brief notice on subsequent invocations. The app never downloads an update automatically. Set `LAML_NO_UPDATE_CHECK=1` to disable the background request and notices.

## Build

```sh
cargo run -- check examples/architecture.laml
cargo run -- build examples/architecture.laml
# writes examples/architecture.svg
cargo run -- build examples/architecture.laml examples/architecture.png
cargo run -- build examples/architecture.laml examples/architecture.pdf
```

`build` accepts an optional output path: `laml build FILE.laml [OUTPUT.svg|OUTPUT.png|OUTPUT.pdf]`. An omitted output path produces SVG. `check` validates syntax, references, styles, and directional placement cycles. `build` additionally validates measured geometry and routing before writing the SVG. If the mandatory rules or safe routing cannot be satisfied, it reports an error instead of writing an invalid image.

SVG defaults to embedded Inter Regular with selectable text; diagrams with abstract UML text also embed the matching Inter Italic face. `--svg-text=system` produces selectable SVG text without embedding the font; `--svg-text=paths` turns SVG text into glyph outlines. PNG uses a white background; PDF is vector and retains selectable text. The bundled Regular and Italic font faces share the license at [assets/INTER-LICENSE.txt](assets/INTER-LICENSE.txt).

## Examples

| [Architecture](examples/architecture.laml) | [Crossing](examples/crossing.laml) |
|---|---|
| [![Twelve-node architecture across channel, gateway, service and data layers](docs/images/architecture.png)](examples/architecture.laml) | [![Two relations crossing with a bridge](docs/images/crossing.png)](examples/crossing.laml) |
| Twelve nodes, four layers, service calls, local LSS import and mandatory layout. | A supported crossing with a bridge. |

| [Parallel relations and loop](examples/parallel-loops.laml) | [Relation-label modes](examples/label-modes.laml) |
|---|---|
| [![Service and broker with parallel relations, response arrow, and retry loop](docs/images/parallel-loops.png)](examples/parallel-loops.laml) | [![Inline Batch label and angled Stream label between two node pairs](docs/images/label-modes.png)](examples/label-modes.laml) |
| Distinct ports and an exterior self-loop. | Inline text with a per-relation along-path override. |

| [Sequence: calls and replies](examples/sequence-basic.laml) | [Sequence: alternative lifetimes](examples/sequence-alternative-births.laml) |
|---|---|
| [![Browser request, reply and interaction reference](docs/images/sequence-basic.png)](examples/sequence-basic.laml) | [![Alternative operands create and destroy the same Resource lifeline](docs/images/sequence-alternative-births.png)](examples/sequence-alternative-births.laml) |
| Lifelines, executions, messages and a `ref` frame. | The same participant ID has separate lifetime segments in two operands. |

| [Classes and association ends](examples/class-basic.laml) | [Object instances](examples/object-basic.laml) |
|---|---|
| [![Two classes with members, association-end multiplicities and roles, and an attached note](docs/images/class-basic.png)](examples/class-basic.laml) | [![Two underlined object instances connected by a link with end names](docs/images/object-basic.png)](examples/object-basic.laml) |
| Typed classifiers, compartments, ends and note-to-edge attachment. | Underlined instances, slots and a link with end names. |

| [Named n-ary association](examples/class-nary.laml) | [Nested packages](examples/class-packages.laml) |
|---|---|
| [![Three classifiers connect to one named association diamond](docs/images/class-nary.png)](examples/class-nary.laml) | [![Nested package frames with enclosed classes and routed associations](docs/images/class-packages.png)](examples/class-packages.laml) |
| One association diamond, three arms. | Scoped names and contained geometry. |

| [Template binding](examples/class-binding.laml) | [Generic graph icons](examples/generic-icons.laml) |
|---|---|
| [![A dashed template binding substitutes T with String](docs/images/class-binding.png)](examples/class-binding.laml) | [![Generic nodes with one folder icon and a visibility icon](docs/images/generic-icons.png)](examples/generic-icons.laml) |
| Checked local substitution. | Optional Phosphor glyphs with conditional export attribution. |

| [Compact provided interfaces](examples/class-lollipops.laml) |
|---|
| [![Two classes each realize a separate API lollipop](docs/images/class-lollipops.png)](examples/class-lollipops.laml) |
| One empty interface, two distinct compact realizations. |

Set `type: sequence` in `settings` to use the sequence grammar. Sequence diagrams use the same SVG/PNG/vector-PDF export commands; their node, message, fragment, annotation, gate and ordering visuals are styled with LSS. Start with the [sequence-diagram guide](docs/sequence-diagrams.md), then use the [syntax reference](docs/language-reference.md#sequence-diagrams) and [UML 2.5.1 notation audit](docs/uml-sequence-conformance.md). The outer UML `sd` interaction frame is intentionally deferred; divergent open executions at fragment joins are rejected rather than implicitly resolved.

Build any example with `cargo run -- build examples/NAME.laml`. The output filename selects the format (`.svg`, `.png`, or `.pdf`). SVG text modes are selected with `--svg-text=embedded|system|paths` after the optional output filename. The README images are generated with `cargo run -- build examples/NAME.laml docs/images/NAME.png`. Source files and matching images are tested together.

## Language feature contract

- Line-oriented `.laml` grammar for typed or generic graph nodes, named or unnamed directed/undirected relations, styling classes, labels, and diagram settings.
- Deterministic automatic placement; mandatory symbolic/word placement rules with minimum gaps and optional edge/centre alignment.
- Bounded node-safe routing with distinct parallel ports, exterior self-loops, and bridges at supported crossings; unsafe diagrams fail.
- Diagram-wide and per-relation label placement (`near`, `along`, `inline`).
- LSS styling and local recursive imports with a deterministic per-property cascade.
- SVG in embedded-font, system-font, or glyph-path text mode; PNG and vector PDF exports.
- Class and object diagram grammars with checked UML relationship notation, member compartments, instances and slots, associations, packages, qualifiers, notes, templates and addressable ports; unsupported combinations fail rather than silently drawing a different notation.
- Generic node-edge shapes, nested containers, styled members, boundary ports, optional icons, named edge attachments and conditional icon-license metadata.
- Separate sequence grammar with lifelines, calls and replies, creation/deletion, fragments, interaction uses, annotations, coregions, inline part decompositions and mandatory layout constraints.
- `check`, `build`, and `update` commands, source-linked diagnostics, and conformance tests for deterministic and mandatory geometry.

## Releases

Pushing a stable version tag such as `v1.2.0` runs the [release workflow](.github/workflows/release.yml). The tag must match the version in `Cargo.toml` and point to a commit containing the workflow. All platform jobs must pass before publication. The GitHub Release contains archives for Linux x86-64, macOS Intel and Apple Silicon, Windows x86-64, and a `SHA256SUMS` file. Each archive contains the executable, example diagrams, README, and license files (including the separately attributed Inter and Phosphor assets).

Update `Cargo.toml` and `Cargo.lock`, commit those changes, then push the commit and its matching version tag. Moving an existing version tag is not part of the release process.

## Verification

```sh
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
```

The test suite includes buildable examples, export formats, mandatory geometry, route safety, label placement, edit stability, and the 500-line-per-Rust-file rule.

## Documentation

- [Getting started](docs/getting-started.md)
- [Language reference](docs/language-reference.md)
- [Language contract](docs/language-definition.md)
- [Sequence-diagram guide](docs/sequence-diagrams.md)
- [Class and object diagram guide](docs/class-object-diagrams.md)
- [UML class/object notation audit and limits](docs/uml-class-object-conformance.md)
