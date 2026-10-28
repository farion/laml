# LAML

LAML is a diagram language and Rust compiler. A `.laml` source file produces one SVG, PNG, or PDF image. Relations describe the diagram; mandatory layout rules let the author control node positions without manually drawing them. Styling lives in LAML Style Sheets (LSS), either inline or in local imported files.

## Build

```sh
cargo run -- check examples/architecture.laml
cargo run -- build examples/architecture.laml
# writes examples/architecture.svg
cargo run -- build examples/architecture.laml examples/architecture.png
cargo run -- build examples/architecture.laml examples/architecture.pdf
```

`build` accepts an optional output path: `laml build FILE.laml [OUTPUT.svg|OUTPUT.png|OUTPUT.pdf]`. An omitted output path produces SVG. `check` validates syntax, references, styles, and directional placement cycles. `build` additionally validates measured geometry and routing before writing the SVG. If the mandatory rules or safe routing cannot be satisfied, it reports an error instead of writing an invalid image.

SVG defaults to embedded Inter Regular with selectable text. `--svg-text=system` produces selectable SVG text without embedding the font; `--svg-text=paths` turns SVG text into glyph outlines. PNG uses a white background; PDF is vector and retains selectable text. The bundled font license is at [assets/INTER-LICENSE.txt](assets/INTER-LICENSE.txt).

## Examples

- [Architecture](examples/architecture.laml) — local LSS import, styling, and mandatory layout.
- [Crossing](examples/crossing.laml) — a supported crossing with a bridge.
- [Parallel relations and loop](examples/parallel-loops.laml) — distinct ports and an exterior self-loop.
- [Relation-label modes](examples/label-modes.laml) — inline text with a per-relation along-path override.

Build any example with `cargo run -- build examples/NAME.laml`. The output filename selects the format (`.svg`, `.png`, or `.pdf`). SVG text modes are selected with `--svg-text=embedded|system|paths` after the optional output filename.

## v1 feature contract

- Line-oriented `.laml` grammar for rectangular nodes, named or unnamed directed/bidirectional relations, classes, labels, and diagram settings.
- Deterministic automatic placement; mandatory symbolic/word placement rules with minimum gaps and optional edge/centre alignment.
- Bounded node-safe routing with distinct parallel ports, exterior self-loops, and bridges at supported crossings; unsafe diagrams fail.
- Diagram-wide and per-relation label placement (`near`, `along`, `inline`).
- LSS styling and local recursive imports with a deterministic per-property cascade.
- SVG in embedded-font, system-font, or glyph-path text mode; PNG and vector PDF exports.
- `check` and `build` commands, source-linked diagnostics, and conformance tests for deterministic and mandatory geometry.

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
