# LAML language reference

A `.laml` file describes one diagram. `laml build FILE.laml [OUTPUT.svg|OUTPUT.png|OUTPUT.pdf] [--svg-text=embedded|system|paths]` produces one image or reports an error; the default output path replaces `.laml` with `.svg`. `laml check FILE.laml` validates syntax, references, imported styles, and directional placement cycles without building geometry or an image. `laml --version` reports the installed version; `laml update` explicitly downloads a verified newer release and updates the current executable.

## Document shape and identifiers

A file contains exactly one `semantics` section and at most one each of `settings`, `layout`, and `styling`, in any order. Each section opens with `{` on the heading line and closes with `}` on its own line. Put one declaration or property on each line. Blank lines are allowed, and `//` starts a comment at a token boundary outside a double-quoted string; an unquoted `https://` is not a comment.

```text
settings {
    direction: left-to-right
}
semantics {
    A
    B
    A -> B : message
}
layout {
    A < B
}
styling {
    node[A] {
        font-size: 14
    }
}
```

Unqualified identifiers match `[A-Za-z_][A-Za-z0-9_]*`; nested package paths use `.` and member addresses use `::`. Explicit `@id` replaces a default graph identity and is unique across explicitly named elements. Node labels and style import paths use double-quoted strings with `\n`, `\r`, `\t`, `\"`, and `\\` escapes. Relation text after `:` may be unquoted to the end of its line. Semicolons separate multiple *inline attributes*, never whole declarations.

## Settings

| Name | Values | Default |
|---|---|---|
| `type` | `class`, `object`, `sequence` (omitted for generic node-edge) | Generic graph |
| `direction` | `left-to-right`, `right-to-left`, `top-to-bottom`, `bottom-to-top` | `left-to-right` |
| `naming-convention` | `uppercase-nodes-lowercase-relations` | Off |
| `relation-label` | `near`, `along`, `inline` | `near` |

```text
settings {
    direction: left-to-right
    naming-convention: uppercase-nodes-lowercase-relations
    relation-label: near
}
```

The optional naming convention requires node IDs to start with an uppercase ASCII letter and *named* relation IDs with a lowercase ASCII letter. Without it, either case is accepted. Unknown or repeated settings are errors. `direction` affects automatic placement, not mandatory layout.

## Semantics

```text
semantics {
    Backend(label="Backend API"; style-classes=backend,critical)
    Broker
    mqtt(style-classes=protocol): Backend -> Broker : MQTT
}
```

A node's identifier is used by relations and layout. `label="..."` replaces the visible ID; `style-classes=one,two` assigns styling classes. Unknown or repeated attributes and duplicate node IDs are errors. Node text wraps only at explicit `\n` breaks. A fixed-size generic node truncates overflowing text with an ellipsis at grapheme boundaries; a fixed UML node cannot truncate meaningful classifier text and fails if it is too small. The selectable-text SVG node `<title>` retains a truncated generic node's complete label. A box too small for a line and padding is an error.

Generic relations use `->`, `<-`, `<->`, and undirected `--`. For example, `A -> B : MQTT`, `A <- B`, and `A <-> B` are valid relation declarations; `A <- B` is directed from B to A. A relation is unnamed unless prefixed by `@id`, as with `@mqtt A -> B : MQTT`; a prefix can also contain `style-classes=...` with or without a name. The older `mqtt:` spelling is accepted for existing sources. Text after the final `:` is its visible label. A relation name is required to target that relation with a `relation[name]` selector or layout rule. Unknown endpoints and duplicate relation names are errors. Multiple relations may share endpoints; each needs a distinct route and port. Up to four exterior self-loops per node can be routed; more are rejected.

Directed relations give source-order *preferences* for automatic placement, not hard rules. A later relation that closes a preference cycle remains in the diagram but loses that automatic directional preference. Bidirectional relations express no preferred direction. Mandatory layout rules always take precedence over preferences.

## Class and object diagrams

Set `type: class` or `type: object` in `settings`. Class declarations (`class`, `interface`, `enum`, `datatype`, `association class`), notes, packages, attribute and operation compartments, objects with literal slots, named ports and checked UML operators share graph geometry with generic nodes. Simple author shorthand `int counter` / `+void start(int timeoutms)` renders as UML `counter: int` / `+start(timeoutms: int): void`; unsupported complex signatures fail. The [working class/object guide](class-object-diagrams.md) gives complete buildable sources and the accepted notation. Class associations can specify independent end multiplicities, roles and qualifiers; object links allow end names but reject multiplicities and qualifiers. Unsupported UML combinations are errors rather than generic arrows. Generic graphs can additionally choose `shape: rectangle|ellipse|diamond|note` in LSS; a UML classifier cannot change its meaningful shape through styling.

## Layout

```text
layout {
    Backend < Broker : align=top; gap=160
    relation[mqtt] : label=along
}
```

The alternatives `left of`, `right of`, `above`, and `below` have the same meaning as `<`, `>`, `^`, and `v`. Each placement rule separates the **full node boxes** on its axis. `gap` sets a finite, nonnegative **minimum** in SVG/CSS pixels; the default minimum is 24. The compiler may increase separation to fit a relation label. Without `align`, there is no constraint on the other axis.

| Direction | Valid `align` values | Matched geometry |
|---|---|---|
| left/right | `top`, `center`, `bottom` | Vertical edges or centres |
| above/below | `left`, `center`, `right` | Horizontal edges or centres |

Invalid alignment/direction combinations, unknown node IDs, duplicate or unknown attributes, and contradictory mandatory rules are errors. `check` detects directional placement cycles; `build` additionally checks alignments, actual dimensions, node collisions, and routing.

For class diagrams, `relation[orders].source : multiplicity=above; role=below` and the corresponding `.target` rule fix end-label sides without reversing association semantics. A named relation can also use `relation[orders].source : side=left|right|top|bottom` to require a direct node boundary side (not a member, port, or self-loop), combined with end-label placement in one rule. `port[id] : side=left|right|top|bottom` fixes a named boundary port. Missing or hidden targets and impossible measured placement fail.

`relation-label` in `settings` supplies the diagram default. `relation[name] : label=near|along|inline` overrides one **named, labeled** relation. `near` keeps horizontal text beside a relation, `along` rotates selectable text along a straight segment while keeping it upright, and `inline` leaves a gap in the relation stroke for horizontal text. A missing relation, missing label, duplicate rule, or invalid mode is an error. A mode that cannot be placed safely fails on `build` rather than switching modes.

## Styling with LSS

```text
styling {
    import "architecture.lamlstyle"
    .backend {
        color: rgb(35, 75, 125)
    }
    node.backend {
        padding: 12
    }
    relation.backend {
        width: 2
    }
    node[Backend] {
        font-size: 16
    }
    relation[mqtt] {
        color: "#aa3131"
    }
}
```

Selectors include `*`, `.class`, `node.class`, `relation.class`, `node[ID]`, `relation[ID]`, `node`, `relation`, `member`, `member[kind=field|method]`, `member[ID]`, `port`, and scoped descendants such as `node[Package] node[Child]`. An unmatched selector is allowed. The cascade resolves each property independently: object-specific > typed class > unqualified class > type-wide; later declarations win at equal specificity. The order of names in an object's `style-classes` list does not affect precedence. A known property that does not apply to a matched object has no effect there.

| Property | Value | Applies to |
|---|---|---|
| `fill`, `border-color` | Colour | Nodes |
| `color` | Colour | Nodes and relations |
| `padding`, `min-width`, `min-height`, `height`, `border-width`, `corner-radius` | Finite nonnegative number | Nodes |
| `width` | Finite nonnegative number | Fixed node width or relation stroke width |
| `font-size` | Finite positive number | Nodes, members, ports and relations |
| `display` | `none` | Nodes, members, ports and relations (visible references fail) |
| `shape` | `rectangle`, `ellipse`, `diamond`, `note` | Generic nodes, subject to measured content |
| `symbol-rendering`, `icon-set`, `icon` | `text|icons`, `phosphor`, allowlisted icon name | Visibility glyphs and optional generic node icons |

Colours are `rgb(r,g,b)` with integer channels 0–255, quoted `"#rrggbb"`, or `none`. Numeric values are unadorned pixels, without a `px` suffix. Unknown properties and invalid values are errors, even for unmatched selectors. Default node minimum size is 100 × 50, padding 12, and font size 14. Width and height, when set, are fixed sizes; a conflicting explicit minimum or a box too small for its text and padding fails.

An imported `.lamlstyle` file contains one `styling { ... }` section. Imports contribute rules at their position in the cascade and may contain nested imports. Paths resolve relative to the importing file and must remain within the diagram source's directory tree after symlink resolution. Missing files, escapes, and cycles are errors. LSS imports contain styling, not semantics or layout.

## Sequence diagrams

Select the separate sequence grammar with `settings { type: sequence }` (one entry per line). Files without `type: sequence` retain the graph grammar above. In sequence sources, `direction`, `relation-label`, and graph routing settings do not apply. `semantic-check: off` disables only the numeric `min <= max` loop-bound check; it never relaxes reference or mandatory layout checks.

```text
settings {
    type: sequence
}
semantics {
    Client
    API(selector="u"; type="Service")
    request: Client -> API+ : query
    <-- : result
    checkout: alt "cache hit" {
        Client ->> API : notify
    }
    else "miss" {
        Client ->> API : refresh
    }
}
layout {
    Client < API : gap=48
    checkout covers Client,API
}
```

Participants draw a head and dashed lifeline; declaration alone does not start execution. `+` starts a bar, `-` ends it, `!` ends the lifeline with X. `A -> B!` and `B! <- A` draw deletion messages. An undeclared target with `+` first introduces a lifeline; `kind=create` in a message prefix chooses the dashed, open creation arrow. `?` is a found/lost message endpoint. `->`/`<-` draw filled synchronous calls, `->>`/`<<-` open asynchronous arrows, and `<--` a dashed reply. A leading `<--` targets the last unambiguous open call; reply arrowheads may be filled with `reply(arrow=filled): <--`. Named messages can be referenced through `.send` and `.receive`. Explicit general ordering can also address a locally named bar: `order[execution(API,work).start,request.receive]` and `order[request.receive,execution(API,work).finish]`. The finish reference requires an explicit earlier close.

`alt`, `opt`, `loop`, `par`, `break`, `strict`, `seq`, `critical`, `neg`, `assert`, `ignore`, and `consider` use brace-delimited fragments. `else "guard" {` separates `alt` operands, `and "guard" {` separates `par` operands. `loop(min=0; max=*; guard="ready") {` displays bounds and a guard without evaluating them. `ignore "m,s" {` displays a curly-braced message list. Adjacent operators such as `strict alt "ready" {` or `opt loop(min=0; max=2) {` share one descriptor as the UML nesting shorthand. Named frames may be constrained in `layout` with `checkout covers Client,API`; all intervening lifelines must be listed. Alternative operands can each create and destroy the same lifeline ID, producing separate visible segments. Divergent open execution bars or lifeline lifetimes at a fragment join fail.

Other visible constructs include `ref "Name(args): result" { ... }` or a combined-fragment block with a named `gate[id]` on its boundary; two opposite-direction messages using the same gate ID in one frame meet one point on that boundary; `at[request.receive] : invariant="ready"` or `action="Read"`; `at[execution(API,work).start] : time-observation="t0"`; `order[first.receive,second.send]`; `coregion API { ... }` with named occurrence ends; and `decompose API "Internals" { ... }` with child lifelines in the same source. Notes and continuations are named in `semantics` and placed in `layout`, for example `hint over API : gap=16` and `wait spans Client,API`. Qualified IDs such as `API.Cache` address an inline decomposition from root layout. LSS descendant selectors such as `node[API] node[Cache]` and `node[API] relation[query]` style only the corresponding descendants. `annotation` and `annotation[action]` style visible annotations; `ordering` styles general-ordering paths and arrowheads; `gate[result]` styles a named gate. Gate styling defaults to its message colour and font size.

Follow the [sequence-diagram working guide](sequence-diagrams.md) for complete, buildable diagrams, then consult the [UML notation audit](uml-sequence-conformance.md) for clause-level coverage and boundaries.

## Output and routing

The output extension selects SVG, PNG, or PDF. SVG defaults to `--svg-text=embedded`: Inter Regular is embedded, and text remains selectable. `--svg-text=system` keeps selectable SVG text but omits the font; the viewer may substitute a font. `--svg-text=paths` converts SVG text into nonselectable glyph outlines and does not preserve node-level `<title>` metadata. These options apply only to SVG output. PNG uses the SVG's CSS-pixel dimensions rounded to whole pixels, on a white background; the image is limited to 64 million pixels. PDF is vector output with embedded, selectable text; 96 SVG pixels map to one inch of PDF page size. Rectangular nodes may have rounded corners via `corner-radius`. Relations attach to box boundaries. The router prefers a straight route and searches a finite set of detours when blocked. Parallel relations use separate boundary ports; self-relations use exterior loops. At a supported crossing, the later-declared relation receives a bridge and the earlier relation a gap. If no safe routing or label placement is found, `build` fails without writing a replacement image: it does not emit bare crossings or silently ignore mandatory rules.

The installers and updater are described in the [README](../README.md). Normal `check` and `build` commands do not wait for the network: a cached version is shown if newer, and a background refresh runs at most once per 24 hours. Set `LAML_NO_UPDATE_CHECK=1` to opt out. See [Getting started](getting-started.md), the [class/object guide](class-object-diagrams.md), the [language contract](language-definition.md), and the [buildable examples](../examples/).
