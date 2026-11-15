# GNE editor inventory

Date: 2026-11-14. This is an implementation inventory, not a new language design.
Sources inspected: `src/styles.rs`, `src/geometry/nodes.rs`,
`src/geometry/node_notation.rs`, `src/geometry/ports.rs`,
`src/parser/{bodies,graph_operators,layout,port_bodies,members}.rs` and the GNE guide.

## Elements and semantics

| Delivered syntax/mechanism | Current visual editing |
|---|---|
| Generic node | Palette creation, source-linked selection, independent label/global-ID editing, styles, arrangement/alignment and deletion |
| `rectangle`, `ellipse`, `diamond`, `note` shapes | Palette and local `shape` inspector; nonrectangular compartment restrictions retained |
| `note ID "text"` / note body | Palette creates quoted semantic text; label inspector edits inline/block notes |
| `package ID { ... }` | Palette creation, nested creation, label/ID editing, membership proposals, both deletion modes |
| Generic/class boundary ports | Creation in an existing body or by converting a leaf into a body; selection, source linkage, sides, name/global-ID editing, styles and safe deletion |
| Relations to node/port endpoints | Connect gesture and endpoint-handle reconnection; semantic direction remains separate from docking |
| Relation operators | GNE operator list from existing parser; dash/marker choices are operators, not an invented `line-style` property |
| Relation labels | Text editing, `near/along/inline` overrides and reset to diagram default; mode controls require labels |
| Node/port/relation style classes | Effective cascade plus node/relation/port declaration inspectors for class-list authoring |
| Member/compartment bodies | Previewed body editing with applicable row snippets, member ID/reference updates, effective member styles and compartment visibility |
| Note-to-relation attachments | Address-based creation/removal with preview, selectable links/source linkage, and attachment-only confirmed deletion |
| Specialized classifier/object/template notation permitted in GNE source | Class/interface/enum/datatype/object/junction palette; previewed full declaration fields retain identity and validate compiler topology |

All palette IDs are checked against existing addresses and scoped default aliases.
Generated global port IDs and nested node names must not capture an existing
reference. Creation checks normalized endpoints before and after the patch.

## Local property inventory

| Element | Current inspector properties |
|---|---|
| Node | `fill`, `border-color`, `border-width`, `color`, `base-color`, `font-size`, `padding`, `width`, `height`, `min-width`, `min-height`, `corner-radius`, `shape`, `icon`, `symbol-rendering`, `icon-set`, `display` |
| Port | `fill`, `border-color`, `color`, `base-color`, `display` |
| Relation | `color`, `base-color`, `width`, `font-size`, `display` |
| Member | `color`, `font-size`, `display`, `symbol-rendering`, `icon-set` |
| Compartment | `display` |

The compiler's public `styles::property_value` supplies validation/normalization;
editor code does not maintain another color/numeric parser. Icon source values are
quoted safely. The icon browser shares the bundled compiler catalogue.

Effective snapshots use the compiler's scoped cascade and rendered geometry/paint.
Imports, contextual specificity, theme tinting, mixed values and local reset are
represented. Missing concrete values show automatic/unset rather than fabricated
numbers. Hidden elements remain discoverable through the source element list.

Applicability is not identical to parser acceptance: a package cannot change its
rectangular form; additional icons apply to generic nodes; ellipse/diamond options
are disabled when selected nodes contain member compartments. Full compilation is
still the final authority on sizes, contours, ports and routes.

`interface-notation` is notation-specific rather than a generic node decoration;
`length` belongs to activity bars. Port font size follows the owner; it is not an
applicable port LSS property. Diagram-level theme, direction, fonts, imported styles
and defaults remain editable in source rather than invented per-object fields.

## Ports and names

Port names are identifiers, not free-form quoted labels. The existing syntax has
no independent `label=` port attribute. A default port address (`Owner::Name`) is
therefore name-derived. Its name control stays disabled until the user explicitly
assigns a global address; that deliberate ID transaction updates references while
retaining the visible name. Subsequent name edits retain the explicit ID.

The implementation reuses `port[id] : side=...` and
`relation[id].source|target : side=...`; reset removes only the relevant override.
Existing port decorator attributes, end-label attributes and comments survive.

## Membership and transactional safety

Membership is a separate, single-node semantic action with a revision/selection-
bound proposal and confirmation. Whole declarations move with comments; default
child/port addresses and direct selectors rebase. References inside the moved
scope are made explicit where representable. Contextual selectors remain in their
original scopes, so inherited styling intentionally follows the new ancestry.

Relations, attachments and layout rules are never removed to make membership fit.
Address collisions, descendant cycles and changed endpoint resolution reject the
candidate; full compilation can also reject unsatisfiable geometry/routing. No
new absolute-root reference syntax or editor-only geometry was introduced.

## Source and workflow

Source assistance includes structured line diagnostics, gutter, literal find/replace,
context-sensitive completion, delimiter matching, comment/indent commands, explicit
compiler-model-preserving formatting and source linkage for declarations, layout,
properties and attachments. Shared delta history is bounded and groups typing.

Workflow includes zoom/pan/fit/100%, resizable panels, native/path-based file dialogs,
revision-bound compiler SVG/PNG/PDF export, local stylesheet/font dependency polling,
and private session-locked recovery snapshots with restore/discard.

## Validation and shipping

A–D functionality is implemented locally, with conservative rejection of unsafe
reference changes. Linux native framebuffer capture and the complete workspace
suite are recorded in handover section 14. CI now targets the workspace and builds
CLI/UI separately. The independent editor-release workflow produces four platform
archives, a macOS app bundle and checksums without replacing the CLI latest channel.

Native human acceptance and actual macOS/Windows execution are still required;
workflow definitions are not evidence that those platforms ran. No public editor
release is established. All changes remain local/uncommitted.
