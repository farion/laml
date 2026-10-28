# Getting started with LAML

LAML separates **semantics** (nodes and relations), **layout** (mandatory placement and label rules), and **styling** (appearance). One `.laml` file builds one SVG, PNG, or PDF image. For a prebuilt executable on Linux, macOS, or Windows, use the [installer instructions](../README.md#install-the-latest-release); run `laml update` when you choose to install a newer version.

## 1. Describe the diagram

```text
semantics {
    Client(label="Client application"; classes=external)
    Backend(label="Backend API"; classes=backend)
    Broker(label="Message broker"; classes=infrastructure)

    Client -> Backend : HTTPS
    mqtt(classes=async): Backend -> Broker : MQTT
}
```

`Client`, `Backend`, and `Broker` are node identifiers. `label` supplies visible text; `classes` group objects for styling. Relations can be unnamed or have a name such as `mqtt` for individual targeting. Visible relation text follows the final `:`. `A <- B` means B to A; `A <-> B` is bidirectional. The [complete architecture example](../examples/architecture.laml) uses these declarations.

## 2. Add only the placement rules you need

```text
layout {
    Client < Backend : align=center; gap=24
    Broker below Backend : align=left
}
```

The first rule separates the **boxes** by at least 24 layout units and aligns their vertical centres. The second puts `Broker` below `Backend` and aligns their left edges. These rules are mandatory, not preferences; `gap` is a minimum rather than an exact distance. A directional rule without `align` places no requirement on the perpendicular axis. One layout unit corresponds to one SVG/CSS pixel.

Symbols and words are equivalent: `<` / `left of`, `>` / `right of`, `^` / `above`, `v` / `below`. A contradictory set of mandatory rules fails with a source-line diagnostic. Relations guide automatic placement but never override a mandatory rule.

### Relation labels

The `relation-label` setting chooses a diagram-wide mode: `near` (default), `along` (text following a straight segment), or `inline` (horizontal text in a gap in its own relation). A named, labeled relation can override that setting in `layout`:

```text
settings {
    relation-label: inline
}
layout {
    relation[along] : label=along
}
```

These sections are part of the [buildable label-modes example](../examples/label-modes.laml). If a chosen mode cannot be placed safely, `build` fails instead of choosing a different mode.

## 3. Style separately

```text
styling {
    import "architecture.lamlstyle"

    node.backend {
        fill: rgb(35, 75, 125)
        color: rgb(255, 255, 255)
        font-size: 14
    }

    relation.async {
        width: 2
    }
}
```

LAML Style Sheets (LSS) have CSS-like selectors, but declarations use newline-separated `property: value` entries, without semicolons. `.async` matches a class on either kind of object; `node.backend` and `relation.async` target just one kind. `node[Backend]` and `relation[mqtt]` target individual named objects. Imports load local `.lamlstyle` files relative to the file containing the import.

## 4. Build and inspect

From the repository root:

```sh
cargo run -- check examples/architecture.laml
cargo run -- build examples/architecture.laml
# writes examples/architecture.svg
cargo run -- build examples/architecture.laml examples/architecture.png
cargo run -- build examples/architecture.laml examples/architecture.pdf
cargo run -- build examples/architecture.laml examples/architecture-paths.svg --svg-text=paths
```

`check` validates the source, references, imported styles, and directional placement cycles; `build` also measures boxes, solves placement, routes relations, and writes SVG. Routing uses distinct boundary ports for parallel relations, exterior self-loops, and bridges for supported crossings. If no safe route is found, the build fails rather than drawing an overlapping or bare-crossing stroke. The default SVG embeds Inter Regular and retains selectable text. The `--svg-text=system` option omits the font while keeping text selectable (the viewer must provide Inter or substitute another font); `--svg-text=paths` turns text into nonselectable glyph outlines. PNG has a white background. PDF is a vector document with selectable text.

Other buildable examples: [crossing](../examples/crossing.laml), [parallel relations and loop](../examples/parallel-loops.laml), and [label modes](../examples/label-modes.laml).

## 5. Syntax reminders

- Write one declaration or property per line. Use `;` only between inline attributes, such as `label="API"; classes=backend`.
- Identifiers match `[A-Za-z_][A-Za-z0-9_]*`. Node labels and import paths are double-quoted; strings support `\n`, `\r`, `\t`, `\"`, and `\\` escapes. A relation label can extend unquoted to the end of its line.
- `#` begins a comment outside a quoted string. For hexadecimal colours, use quoted `"#rrggbb"` (or use `rgb(r,g,b)`).
- Named relations are required for per-relation layout and styling; endpoints must refer to declared nodes.

See the [language reference](language-reference.md) for the full syntax and the [language contract](language-definition.md) for compiler behaviour.
