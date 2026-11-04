# Component diagrams

## Basic example

```laml
settings {
    type: component
}
semantics {
    component Gateway
    component Orders
    @calls Gateway ..> Orders : place order
}
layout {
    Gateway < Orders : gap=120
}
```

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../images/component-quickstart-modern-dark.png">
  <img src="../images/component-quickstart-modern.png" alt="Gateway depends on Orders">
</picture>

[Complete source](../../examples/component-quickstart.laml). `..>` draws a dependency; `layout` requires the components to keep their separation.

## Interfaces

```laml
// Excerpt from examples/component-basic.laml
semantics {
    // […]
    interface OrderAPI
    interface HealthAPI
    @requires Gateway ..> OrderAPI
    @provides Orders ..|> HealthAPI
}
```

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../images/component-basic-modern-dark.png?v=ea7813c">
  <img src="../images/component-basic-modern.png?v=ea7813c" alt="Required and provided component interfaces">
</picture>

[Complete source](../../examples/component-basic.laml). `..>` points to a required interface box; `..|>` is a provided realization. The source chooses `interface-notation: lollipop` for the provided interface.

## Ports and internal connections

```laml
// Excerpts from examples/component-internal.laml
semantics {
    // […]
    component Storefront {
        port Public(requires=OrderAPI)
        part browser(of=Browser)
        part orders(of=Orders)
        @assembly browser::Out == orders::In
        @delegation Storefront::Public => browser::In
    }
}
```

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../images/component-internal-modern-dark.png">
  <img src="../images/component-internal-modern.png" alt="Component with typed ports and internal connectors">
</picture>

[Complete source](../../examples/component-internal.laml).

| Form | Meaning |
|---|---|
| `port P(provides=Interface)` / `port P(requires=Interface)` | Typed boundary port. |
| `partA::out == partB::in` | Assembly: opposite-role compatible ports. |
| `Owner::outer => part::inner` | Delegation: same-role ports. |
| `interface-notation: lollipop` / `socket` | Compact provided / required interface. |

## Provided and required compact interfaces

```laml
// Excerpt from examples/component-interfaces.laml
styling {
    relation[provides] {
        interface-notation: lollipop
    }
    relation[requires] {
        interface-notation: socket
    }
}
```

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../images/component-interfaces-modern-dark.png?v=ea7813c">
  <img src="../images/component-interfaces-modern.png?v=ea7813c" alt="Component with a lollipop and socket">
</picture>

[Complete source](../../examples/component-interfaces.laml). The compact forms are selected on named interface relations; typed ports remain independently addressable.

For common positioning and styling: [GNE](gne.md). [Full syntax](../language-reference.md#component-diagrams).
