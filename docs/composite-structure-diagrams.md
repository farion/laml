# UML composite-structure diagrams

Set `type: composite`. This draws a structured classifier with its *internal structure* compartment. A `part` is owned by composition and receives a solid box; a noncomposite `role` has a dashed box. Both are typed by explicitly declared local classes. A port connector inside the owner is not an association between arbitrary instances and is not a dependency between class definitions. See the [buildable checkout example](../examples/composite-checkout.laml).

```text
settings {
    type: composite
}
semantics {
    interface OrderAPI
    class Gateway {
        port Public(requires=OrderAPI)
        port Backend(requires=OrderAPI)
    }
    class Store {
        port In(provides=OrderAPI)
    }
    structure Checkout {
        port Front(requires=OrderAPI)
        role gateway(of=Gateway)
        part store(of=Store)
        @external Checkout::Front => gateway::Public
        @internal gateway::Backend == store::In
    }
}
layout {
    Checkout.gateway < Checkout.store : gap=160
    Gateway < Store : gap=90
    Gateway above Checkout : gap=85
    Store above Checkout : gap=85
    port[Checkout::Front] : side=left
    port[Checkout.gateway::Public] : side=left
    port[Checkout.gateway::Backend] : side=right
    port[Checkout.store::In] : side=left
}
```

`structure Checkout { ... }` is the root structured classifier; there must be exactly one, with at least one direct `part` or `role` and an explicit connector. The type declarations (`class` and `interface`) are outside its body. The compiler resolves a part/role type to a **declared class**, shows `gateway:Gateway` / `store:Store`, and copies the class's named simple ports to the contained role. A port explicitly names one declared interface using `provides=Interface` or `requires=Interface`; no interface set or compatibility is inferred from an untyped class definition.

`==` is an **assembly connector** between ports on distinct sibling parts/roles, requiring matching interface names with opposite provided/required roles. `=>` is a **delegation connector** from an external port on the owner to a port on its own part/role, requiring matching interface and the same provided/required role. Both draw plain solid connector paths attached to visible port squares, with no arrowhead; the operator expresses source-level connector kind, *not* a dependency or direction of runtime messages. Generic `--`, dashed dependencies, unknown port contracts and end adornments fail. Boundary-side and separation rules are mandatory; unsafe routing and connector crossings through the *internal structure* label fail rather than changing UML meaning.

`check` validates identity, ownership and notation; `build` validates measured containment, label clearance and routed geometry, then produces SVG, PNG or vector PDF. See the [UML 2.5.1 notation audit](uml-composite-structure-conformance.md) for connector notation.
