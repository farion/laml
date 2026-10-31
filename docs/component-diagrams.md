# Component diagrams

Set `type: component` in `settings`. This produces an image of a documented UML notation subset, not a component metamodel or runtime wiring simulator. The notation follows OMG UML 2.5.1 §§11.2, 11.3 and 11.6. The source files in [`examples/`](../examples/) build as SVG, PNG and vector PDF.

## Overview and interface contracts

The [component overview](../examples/component-basic.laml) distinguishes a component dependency (`..>`) from an interface realization (`..|>`). `Component ..> Interface` shows a *required* interface in box form; `Component ..|> Interface` shows a *provided* interface in box form. Components display `«component»` and a measured UML component glyph in a classifier rectangle. A top-edge port takes precedence over the optional corner glyph so the two cannot overlap. Interface boxes can contain the supported operations and attributes. Packages, notes, source-scoped IDs and LSS reuse the graph core.

A relation-specific `interface-notation: lollipop` turns an unadorned, named component-to-interface realization into a labeled provided-interface ball. `interface-notation: socket` similarly turns a component-to-interface dependency into a labeled required-interface socket. The box may disappear only if it has no other visible uses, members, ports or mandatory placement. An interface referenced in both roles stays a visible box; selecting compact notation for just one relation fails instead of silently hiding the other's endpoint. See [compact interfaces](../examples/component-interfaces.laml).

## Ports, parts and wiring

A component may declare `port In(requires=OrderAPI)` or `port Out(provides=OrderAPI)`. The value must resolve to an interface declared in the diagram. The contract is visible alongside the measured port label; `port[Component::In] : side=left` can mandate its boundary side. Untyped GNE and class ports keep their existing behavior; UML component ports require an explicit contract.

A containing component uses `part browser(of=Browser)` to name a part typed by a declared component. Its simple ports come from that component's declaration. Assembly `partA::Provided == partB::Required` joins **distinct sibling parts**, with opposite roles for the same interface ID. Delegation `Owner::External => part::Internal` joins the owner's boundary port to a port on its own part with the same interface and role. Neither operator is a component dependency; both draw a solid connector between the *visible port squares*. See the [internal wiring example](../examples/component-internal.laml):

```text
settings {
    type: component
}
semantics {
    interface API
    component Client {
        port In(requires=API)
        port Out(requires=API)
    }
    component Service {
        port In(provides=API)
    }
    component System {
        port External(requires=API)
        part client(of=Client)
        part service(of=Service)
        @assembly client::Out == service::In
        @delegation System::External => client::In
    }
}
```

The defining components and the containing component are independent classifier declarations. Only the named parts inside `System` are its internal structure; a top-level dependency between `Client` and `Service` would **not** declare their assembly. Port connections can use the shared mandatory side and separation rules. A fixed-size component that cannot enclose its parts fails.

## Notation limits

Only simple one-interface ports and binary assembly/delegation connectors are validated. LAML does not infer port compatibility from operation signatures, implement complex ports with several interfaces, show n-ary channelled ball/socket assembly notation, or execute runtime requests. The assembly line attaches to port squares, not an optionally drawn ball-and-socket connector glyph. White-box `realizations`/`artifacts` compartments, and a general UML Component metamodel are outside this image subset. Unknown types, mismatched contracts, wrong ownership, mixed compact/box uses and unsupported relationship glyphs fail explicitly rather than being drawn as a different UML relationship.
