# LAML language contract

The [language reference](language-reference.md) defines the syntax; this document records the behavioural contract of the compiler. A `.laml` file and its local LSS imports are the inputs for one SVG, PNG, or PDF image. There is no layout history file, hidden placement state, or page model. The canvas encloses the diagram.

## Separation of concerns

- `semantics` declares rectangular nodes, directed or bidirectional relations, identifiers, labels, and classes. A relation is not itself a mandatory spatial constraint.
- `layout` declares mandatory node placement and per-relation label placement. Impossible rules fail; they never become hints.
- `styling` controls size, text, fill, border, and stroke through LSS rules and local imports. Styles affecting node dimensions are applied before placement.
- `settings` selects automatic flow direction, an optional identifier convention, and the default relation-label mode.

The bullets above describe graph diagrams (files without `type: sequence`). A sequence diagram uses separate participant, occurrence, message and fragment rules while retaining the same separation between semantics, mandatory layout and LSS styling.

Sections may appear in any order; each kind appears at most once, and `semantics` is required. Declarations are line-oriented. A node or named relation can be addressed by its identifier; only named relations can receive individual label-layout or object-specific styling rules.

## Placement

A rule such as `A < B : align=center; gap=24` separates the **boxes** horizontally by at least 24 pixels and aligns their vertical centres. The equivalent word form is `A left of B`. `>`/`right of`, `^`/`above`, and `v`/`below` behave analogously. Without `align`, the perpendicular axis is unconstrained. The default gap is 24 pixels; `gap` is a minimum, not an exact distance.

Node dimensions are measured with bundled Inter Regular before solving placement. The default minimum box is 100 × 50 pixels with 12 pixels of padding and 14-pixel text. Explicit line breaks determine node text lines; fixed boxes ellipsize overflow at grapheme boundaries. A box too small for one line and its padding fails. Node boxes must not overlap in a successful image.

Directed relations guide *automatic* placement in declaration order. A later relation closing a directional-preference cycle loses that preference, but remains a relation to draw. Explicit rules take priority over automatic preferences; they cannot be removed to make a diagram fit. With neither relations nor placement rules, nodes are arranged top-to-bottom in declaration order. The same source, styles, and compiler environment yield the same placement. Editing source can change automatic positions; no previous render is consulted.

## Routing and label placement

Relations attach at node-box boundaries. The router prefers straight segments, then searches a bounded set of detours. Parallel relations use distinct ports and source-order lanes; self-relations take exterior routes (at most four per node). A supported crossing has a bridge on the later-declared relation and an underpass gap on the earlier one. A bare crossing, overlapping route, blocked port, or unsafe bridge is not a successful image: if the bounded search cannot find a safe arrangement, compilation fails.

Relation text defaults to `near` (horizontal beside a route). `along` rotates text along an eligible straight segment while keeping it upright. `inline` places horizontal text in a gap in its own stroke. The `relation-label` setting chooses the diagram default, and `relation[name] : label=...` in `layout` selects one named, labeled relation. Label placement searches avoid node boxes, unrelated strokes, and other labels; a requested mode is not silently replaced with another.

## Sequence-diagram contract

`settings { type: sequence }` selects sequence parsing. Source order provides a stable visible row tie-break, not an assertion that independent occurrences on different lifelines are temporally ordered. Named message `.send`/`.receive` ends anchor annotations, ordering links, notes and coregions. A mandatory `above : gap=...` coregion rule can reorder occurrences and expands later rows, execution bars, notes and enclosing fragment frames. Cycles, upward-pointing messages and impossible frame coverage fail. The same source and LSS produce the same geometry without previous-render state.

Fragments show literal guards and bounds; no guard or event trace is executed. Every operand must leave the same open executions and lifeline lifetime state. A birth and death can occur within one operand; the same participant ID can have separate lifetime segments in alternative operands. If the operands diverge at the join, build/check fails rather than inventing an endpoint. Inline part decomposition uses child lifelines in the same source, with qualified root-layout IDs. `ref` labels and gate names are visible text; they do not load or infer another UML model. The outer `sd` frame is deferred beyond this diagram type. [Sequence notation and syntax](language-reference.md#sequence-diagrams) and [UML audit](uml-sequence-conformance.md) give the details.

## Styling and output

LSS selectors match types, classes, or named objects. For each property, object-specific rules outrank typed-class rules, which outrank unqualified-class rules, which outrank type-wide rules; later equally specific declarations win. Local `.lamlstyle` imports contribute rules at their position and cannot escape the source diagram's directory tree, including through symlinks. This cascade is not CSS.

`laml check` validates parsing, references, LSS, and directional-rule cycles. `laml build` additionally measures, solves geometry, routes, and exports SVG, PNG, or PDF. One geometry/routing result drives every format. SVG defaults to embedded Inter Regular and selectable text; `--svg-text=system` keeps selectable text without embedding the font, and `--svg-text=paths` converts text to glyph outlines. PNG renders on white at the SVG dimensions rounded to whole pixels. PDF uses vector shapes and embedded selectable text, with 96 layout units per physical inch. It writes the output only after all phases succeed. A build error leaves an existing output image untouched.
