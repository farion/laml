# GNE, class and object diagrams — implementation contract (design, not yet shipped)

This document records the agreed v2 design so implementation can proceed in vertical, tested slices. Existing `docs/language-reference.md` describes the *currently implemented* language; none of the new examples below should be represented as working until the corresponding parser, validation, layout, routing, styling and three exporters pass tests. Do not publish partially implemented UML as conformant.

## Separation and defaults

- `settings { type: class }` and `type: object` select defaults and UML notation checks. No `type` means generic node-edge (GNE). GNE accepts freely ordered node contents; a specialized UML type rejects ill-formed notation. Do not infer an unseen UML model or run code.
- GNE owns shapes, containers, compartments, ports, addressable members, edge-end ornaments and labels, node-to-edge attachment, geometry, routing, collision handling, LSS, and export. UML-specific parsing and validation reuse these primitives rather than duplicate renderers.
- Source alone determines geometry. Mandatory layout rules fail when impossible. Undirected relationships contribute weak topological proximity, not a semantic direction or PlantUML dash-count layout hint.
- `//` is the only source comment outside strings, at line start or after whitespace; never mistake unquoted `https://` for a comment. Migrate *all* `.laml`/`.lamlstyle` examples, tests and documentation from `#` comments. `#` means protected visibility where applicable. Do not change quoted `"#rrggbb"` colors.

## Declarations and identity

```laml
settings {
    type: class
}
semantics {
    abstract class Dienst(style-classes=domain) <<service>> {
        static int counter
        --
        abstract +void start(int timeoutms)
    }
    interface API {
        +void senden()
    }
    enum Status {
        OFFEN
        BEZAHLT
    }
    datatype Geld {
        betrag: Decimal
    }
    @realisiert Dienst ..|> API
}
```

- Bare words before the name identify an element kind or significant modifier (`class`, `interface`, `object`, `anonymous`, `abstract`, `static`, `field`, `method`, `note`, `package`, `association`, `port`). `class`/`object` may be omitted when they are the diagram default.
- `(...)` after an element's name contains named extra values, e.g. `of=Kunde`, `label="..."`, `style-classes=domain,important`. Rename existing `classes=` to `style-classes=` everywhere, including sequence diagrams and imported styles. LSS `.domain` is a styling class, not a UML class or a `<<stereotype>>`.
- `<<stereotype>>` is visible UML-style notation on nodes or relations, separate from a free label and styling classes. Free stereotype names do not imply a loaded profile.
- Elements with natural names receive default IDs in their containing scope; ambiguity is permitted for multi-match styling but fails for a single-target reference (layout, edge endpoint, note attachment). Members with the same name in different classes can have default IDs in their respective scopes. Method overloads with the same default name are ambiguous when targeted.
- `@id` replaces the default ID and is diagram-wide unique across explicitly named elements. It never changes the visible name. It is supported consistently for nodes, members, ports and relations. A target can use a qualified default path such as `Vertrieb.Bestellung::id` or a global explicit ID. Moving an `@id` element between containers does not change its address.
- Styling classes and identities are independent: `node[Bestellung]` selects matching *default IDs*, not manually aliased elements of the same visible name; `.bestellung` or `node.bestellung` selects elements tagged `style-classes=bestellung`, including aliases. A named element has exactly one ID, not an implicit second alias.
- The relation declaration prefix `name:` is replaced with `@name` before v2. Existing files and docs can be migrated without compatibility aliases.

## Nodes, members, notes and objects

```laml
semantics {
    Kunde {
        @kundenId id: UUID
        name: String
        --
        +void bestellen()
        text "--" // literal body line; not a separator
    }
    note Hinweis(style-classes=warning) "Nur aktive Kunden"
    Hinweis .. Kunde
}
```

- The node name/label is a header, separated from a nonempty body automatically; `--` separates compartments *within* the body. GNE permits arbitrary separators, arbitrary ordering, and literal `text "..."`. In `type: class`, a mixture of attributes and operations requires an explicit separator; wrong-compartment entries fail rather than being silently moved. Only attribute and operation compartments are initially recognized; UML receptions/internal structure require separately named future grammar.
- Distinguish fields and methods with parentheses; front words `field` / `method` override the heuristic. Do not parse an entire programming-language signature. Visibility `+ - # ~` is optional, including on classifiers and association-end roles. `abstract`/`static` are front modifiers, not `{abstract}`/`{static}`.
- A note's ID is not drawn. Text immediately after its declaration is its visible content. Multiline note blocks hold text, not graph compartments. Notes may attach to a node, an addressable member or a named relation.
- `settings { type: object }` defaults to instances: `kunde1(of=Kunde) { name = "Ada" }` has an underlined `kunde1:Kunde` header. Both instance name and `of` are optional individually. An anonymous object must be explicit, e.g. `@gast anonymous object(of=Kunde)` draws `:Kunde`; it must have a visible classifier and a stable `@id`. Object slots use `name = value` syntax but their values are not evaluated. A link between instances is not a class association; do not put multiplicities on object links per the user's UML check. Do not require an external class model to render an instance.
- Full fixed-width text in GNE can ellipsize, including member text; an undersized fixed UML node fails rather than hiding a name or member. A too-small fixed container fails in *every* type: whole child nodes are never elided.
- An explicit LSS `display: none` can hide a member, compartment, node, or relation. Hide no still-visible reference to it: fail if a visible edge or mandatory layout rule targets an invisible element. Avoid dangling edges or silently deleting them.

## Relations and attachments

```laml
semantics {
    @orders Kunde "1"/kunde -- "0..*"/+bestellungen Bestellung : bestellt
    @owns Bestellung "1" *-- "1..*" Position
    @extends PremiumKunde --|> Kunde
    @realizes Dienst ..|> API
    @use Client ..> Service <<use>> : ruft auf
    note Hinweis "Genehmigung erforderlich"
    Hinweis .. relation[orders]
}
```

- PlantUML-like operators encode *visible line and end symbols*, not dash-count layout hints. `--` leaves navigability unspecified; `-->` is a solid navigable association and `..>` a dashed dependency; `--|>` and `..|>` distinguish generalization and realization. Composition and aggregation use the appropriate diamond at the whole end. GNE can draw the symbols without pretending every GNE relationship has UML meaning. A UML diagram rejects LSS overriding a semantically meaningful UML line pattern or end symbol.
- The quoted strings at an end denote UML multiplicity in `type: class`; `/role` is a separate end label; role visibility is optional. GNE allows free end text. End-label placement is automatic with optional mandatory `relation[id].source/target` constraints for role and multiplicity side; colliding impossible placements fail.
- Relations can target a member ID (attach to its row at the actual node boundary), a named port on a node boundary, or a named *relation*. A relation-to-relation anchor is a general GNE primitive used by notes and UML association classes. Edges connecting child nodes across their own container border cross that border normally, without a bridge or endpoint there; unrelated containers remain obstacles. Bridges remain for crossings between unrelated edges.
- Optional mandatory layout can select a relation's source/target port side; no setting alters semantic arrow direction. Endpoints attach to the visible contour of the chosen shape, not just the rectangular bounding box.
- A n-ary UML association uses a central small diamond with three or more arms; GNE owns its general junction geometry. This is not three independent binary associations or a composition diamond at one end.
- Interfaces can be rendered compactly as lollipops by *relation*-specific styling of a realization, not by blindly changing the interface node shape. A lollipop may replace an otherwise unused and empty interface box; declared members or mandatory layout referring to the box make that compact rendering an error. Multiple providers may each show their own circle.
- Template input such as `class Repository<T>` displays inline `Repository<T>` by default in GNE and class diagrams. In the UML variant parse the parameter names so an explicit `<<bind>>` relation can validate names of locally declared template parameters; no hidden type system. Audit the exact UML binding-argument notation before implementing that specialized syntax.
- Do not freely mix class and instance elements in a UML-specialized diagram until a notation audit validates the particular combinations. GNE permits mixing.

## Styling, symbols and exports

- Extend LSS with lowest-specificity `*`, `member`, `member[kind=field|method]`, `member[ID]`, `member.<style-class>`, `port`, and contextual selectors. Named ID rules beat type/style-class rules. Type selectors are *not* automatically injected styling classes.
- `shape: ...` is a GNE LSS property; UML element kinds constrain shapes that convey their semantics. Notes, package/container frames, qualifiers, junction diamonds, members, ports and icon slots reuse one measured graph geometry pipeline.
- Default visibility uses glyphs. LSS can opt into Phosphor icon rendering globally with `* { symbol-rendering: icons; icon-set: phosphor }` and override an individual member to text. A separate `icon: "folder"` decorates a GNE node; UML limits additional icons to places that remain meaningful and notation-safe. Visual changes do not change element type.
- LAML remains MIT-licensed. Bundle the Phosphor MIT copyright/license notice with any vendored assets. Embed the notice as nonvisible metadata in SVG, PNG and PDF **only when an actual Phosphor icon occurs in that output**. If reliable metadata embedding fails for an output format, fail rather than silently omit the notice. The exact export-notice policy merits legal review.
- A styled and routed graph must drive SVG, PNG and PDF from the same geometry. No partly written output after error; keep font/icon metrics deterministic. All Rust source files remain at most 500 lines (see `AGENTS.md`).

## Implementation order

1. Introduce and test the v2 lexical/identity AST contract and migration of comments, `style-classes`, and relation `@id`; preserve sequence behavior by deliberately migrating its source/examples as well.
2. Extend GNE node measurement/rendering with compartments and member formatting; add scoped/global references, LSS member selectors and fixed-size rules. Validate and render across SVG, PNG, PDF before layering new notations.
3. Add edge-end symbols and independently placed labels, port/member/edge anchors, shapes, containers and their obstacle/routing rules. Keep original graph constraints working.
4. Add class/enum/interface/datatype/package/n-ary/qualifier/association-class/notes/templates as thin validators and notation presets on the same graph. Add object instances/slots/links on the same renderer.
5. Add visibility icon option and conditional Phosphor asset/license metadata, explicit hiding, and full docs/examples; test cross-format geometry, failure cases and determinism. Run all unit/integration/doc examples and strict Clippy before declaring complete.

The v1/v1.2 documents and examples remain descriptions of shipped behavior until these slices are built. This design is not a release claim.
