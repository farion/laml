# Class and object diagrams

Use `settings { type: class }` for class notation or `settings { type: object }` for instance notation. The existing generic node-edge mode has no `type` setting. All three modes share `semantics`, mandatory `layout`, and LSS `styling`; the compiler exports the same measured diagram as SVG, PNG or vector PDF. A rejected construct does not become a different UML relationship automatically.

## A class diagram

The source of [the class example](../examples/class-basic.laml) shows two classifier compartments, typed attributes, operations, association-end multiplicities and roles, and a multiline note attached to an association:

```text
settings {
    type: class
}
semantics {
    class Kunde {
        @kundenId id: UUID
        name: String
        --
        +void bestellen()
    }
    class Bestellung {
        nummer: String
        --
        +void abschliessen()
    }
    @orders Kunde "1"/kunde -- "0..*"/+bestellungen Bestellung : bestellt
    note Regel {
        Eine Bestellung gehört
        zu genau einem Kunden.
    }
    Regel .. relation[orders]
}
layout {
    Kunde < Bestellung : align=center; gap=270
    Regel above Kunde : gap=105
}
```

Front modifiers `field`, `method`, `abstract`, and `static` may appear in any order before a member; duplicate or contradictory modifiers fail. Simple author shorthand such as `static int counter` and `+void bestellen(int timeoutms)` renders as UML-style `counter: int` and `+bestellen(timeoutms: int): void`. The original source is not displayed verbatim. Unsupported or ambiguous parameter forms fail rather than being shown as UML notation. `--` inside a class separates its attribute and operation compartments. `--` between two classifiers declares an association with unspecified navigability. `-->` draws a navigable association with an open arrow; `..>` a dashed dependency; `--|>` a generalization with a hollow triangle; `..|>` a dashed realization. `*--` and `o--` put a composition or aggregation diamond at the whole end. End multiplicities (`"1"`, `"0..*"`), roles (`/kunde`, `/+bestellungen`) and qualifiers (`qualifier(id:UUID)`) are separate association-end annotations. A link in an object diagram may show end names, but never association-end multiplicities or qualifiers.

A named relation can be styled using `relation[orders]`, have end-label sides fixed using `relation[orders].source : multiplicity=above; role=below` in `layout`, or have its actual boundary side fixed with `relation[orders].target : side=left`. The attributes can share a rule. A side rule can constrain a direct node endpoint, a named boundary port, or the left/right boundary of a member row. Conflicting rules on a shared port fail. Named self-loops can set source and target sides independently; their route stays outside the node. An impossible route fails. Named relations can also be the target of a dotted note attachment. A member explicitly named with `@kundenId` can be targeted by an edge. An unaliased member can be targeted as `Kunde::name`; ambiguous overloads fail. An `@id` replaces a default ID and is unique across explicitly named graph elements.

`abstract class Dienst` displays `{abstract}` alongside an actual bundled Inter Italic face; its font-independent annotation also remains visible in exporters that cannot display italics. `interface`, `enum`, `datatype`, and `association class` have checked classifier notation. An association class attaches with `AssociationClass .. relation[relationId]` to one named, plain binary association. A n-ary association uses one small diamond (`association Verordnet`) and at least three distinct plain arms; its visible name stays beside, not inside, the diamond. An association class can instead attach directly by `facts .. Verordnet` to the named diamond; both displayed names must match. One association cannot own two association classes. See [the n-ary example](../examples/class-nary.laml). `class Repository<T>` displays a compact template heading; `StringRepository ..> Repository <<bind(T->String)>>` checks a locally declared parameter and draws its substitution. See [the binding example](../examples/class-binding.laml). `class Repository<T>` is shorthand in source; in a class diagram its formal parameter list appears in a measured dashed rectangle at the classifier's upper-right corner. Optional local formal metadata such as `<T:Class=String, U:DataType>` is shown in that box; only formal names are checked by bindings, not a hidden type system. Generic graphs still use an inline heading. A `relation[id]` style rule with `interface-notation: lollipop` is a relation-specific option for a named `Class ..|> Interface` realization: each provider draws its own labeled ball and solid stalk instead of showing the unused interface box. An interface with members, ports, another kind of relation, a mandatory layout rule referencing its box, or extra relation adornments fails. See [two compact realizations](../examples/class-lollipops.laml).

Package blocks create containing frames and scoped IDs. Inside `package Vertrieb { ... }`, class `Kunde` has default ID `Vertrieb.Kunde`; nested members can be addressed as `Vertrieb.Kunde::name`. A top-level layout rule can use the qualified path, and cross-scope placement separates enclosing packages when necessary. See [nested packages](../examples/class-packages.laml). Fixed-size containers that cannot contain their children fail rather than clipping them.

## Objects

[The object example](../examples/object-basic.laml) declares `kunde1(of=Kunde)` with a slot `name = "Ada"`; its underlined heading reads `kunde1:Kunde`. `@gast anonymous object(of=Kunde)` displays `:Kunde` while retaining a stable explicit address. An object link (`kunde1 /kunde -- /bestellung bestellung1`) can display end names but cannot show multiplicities or qualifier boxes. It is not a class association; its slots are literal text, not evaluated expressions.

## Shared layout and styling

Generic graphs allow arbitrary member rows, `shape: rectangle|ellipse|diamond|note`, and optional node icons. UML classifier shapes cannot be replaced by a styling rule that changes their notation. Class and generic nodes can declare named boundary ports; `port[id] : side=left|right|top|bottom` fixes an addressable port's side. A generic `shape: note` node can use such ports too: a top port is kept clear of the folded corner. Ellipse and diamond ports attach to their actual contours; their labels and headers are checked against the shape and one another, with no rectangular separator lines crossing the silhouette. A contour and content combination with no safe measured port placement fails. For compartments, explicit `width`/`height` must contain UML text rather than ellipsizing it; generic text can ellipsize.

LSS selects nodes, relations, `member`, `member[kind=field|method]`, `member[ID]`, `port`, and styling classes such as `.domain`. `*` is the lowest-precedence selector. A `node[Kunde] compartment[attributes]` style rule with `display: none` suppresses a complete attribute compartment; `compartment[operations]` and `compartment[literals]` target operation and enumeration-literal compartments. Separator lines are removed with the hidden compartment. `display: none` removes an element only when no visible edge or mandatory rule still references it. `style-classes=domain` affects style selection, while `<<stereotype>>` is visible text and has no implicit styling meaning. `//` starts a source comment at a token boundary; `#` is UML protected visibility.

The [generic icon example](../examples/generic-icons.laml) uses `symbol-rendering: icons` with `icon-set: phosphor` and an optional `icon: "folder"`. Phosphor's MIT notice is stored as nonvisible metadata only in SVG, PNG and PDF exports actually using a Phosphor icon; its original notice and provenance are in [assets/phosphor](../assets/phosphor/). A non-icon export contains no Phosphor notice. The font license is separate.

Run `laml check examples/class-basic.laml` for syntax, references and mandatory direction cycles; `laml build examples/class-basic.laml output.svg` additionally checks measured placement and routing. Rendering failures leave an existing output image untouched. See the [syntax reference](language-reference.md) for the shared sections, the [language contract](language-definition.md) for deterministic placement and failure behavior, and the [UML clause audit](uml-class-object-conformance.md) for exact notation boundaries.
