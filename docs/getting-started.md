# Getting started with LAML

LAML separates **semantics** (nodes and relations), **layout** (mandatory placement and label rules), and **styling** (appearance). One `.laml` file builds one SVG, PNG, or PDF image. For a prebuilt executable on Linux, macOS, or Windows, use the [installer instructions](../README.md#install-the-latest-release); run `laml update` when you choose to install a newer version.

## 1. Describe the diagram

```text
semantics {
    Browser(label="Customer portal"; style-classes=channel)
    WebEdge(label="Web gateway"; style-classes=edge)
    Catalog(label="Catalog service"; style-classes=service)
    CatalogDB(label="Catalog store"; style-classes=data)

    Browser -> WebEdge : HTTPS
    WebEdge -> Catalog : query
    Catalog -> CatalogDB : read
}
```

`Browser`, `WebEdge`, `Catalog`, and `CatalogDB` are node identifiers. `label` supplies visible text; `style-classes` group objects for styling. Relations can be unnamed or have a name such as `query: WebEdge -> Catalog` for individual targeting. Visible relation text follows the final `:`. `A <- B` means B to A; `A <-> B` is bidirectional. The [complete architecture example](../examples/architecture.laml) has twelve nodes arranged into four layers.

## 2. Add only the placement rules you need

```text
layout {
    Browser < WebEdge : align=center; gap=110
    WebEdge < Catalog : align=center; gap=110
    Catalog < CatalogDB : align=center; gap=110
}
```

These rules separate each pair of **boxes** by at least 110 layout units and align their vertical centres. The complete example also aligns three lanes with mandatory `above` rules. These rules are mandatory, not preferences; `gap` is a minimum rather than an exact distance. A directional rule without `align` places no requirement on the perpendicular axis. One layout unit corresponds to one SVG/CSS pixel.

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

    node.service {
        fill: rgb(35, 75, 125)
        color: rgb(255, 255, 255)
    }

    node.channel {
        fill: rgb(235, 245, 255)
    }
}
```

LAML Style Sheets (LSS) have CSS-like selectors, but declarations use newline-separated `property: value` entries, without semicolons. `.service` matches a class on either kind of object; `node.service` and `node.channel` target just nodes. `node[Catalog]` and `relation[query]` target individual named objects. Imports load local `.lamlstyle` files relative to the file containing the import.

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

To build lifelines, calls, fragments and annotated occurrences instead, follow the [sequence-diagram working guide](sequence-diagrams.md). Other buildable graph examples: [crossing](../examples/crossing.laml), [parallel relations and loop](../examples/parallel-loops.laml), and [label modes](../examples/label-modes.laml).

## 5. Syntax reminders

- Write one declaration or property per line. Use `;` only between inline attributes, such as `label="API"; style-classes=backend`.
- Identifiers match `[A-Za-z_][A-Za-z0-9_]*`. Node labels and import paths are double-quoted; strings support `\n`, `\r`, `\t`, `\"`, and `\\` escapes. A relation label can extend unquoted to the end of its line.
- `#` begins a comment outside a quoted string. For hexadecimal colours, use quoted `"#rrggbb"` (or use `rgb(r,g,b)`).
- Named relations are required for per-relation layout and styling; endpoints must refer to declared nodes.

See the [language reference](language-reference.md) for the full syntax and the [language contract](language-definition.md) for compiler behaviour.
