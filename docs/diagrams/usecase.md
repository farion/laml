# Use-case diagrams

## Basic example

```laml
settings {
    type: usecase
}
semantics {
    actor Customer
    system Shop {
        usecase Order(label="Place order")
    }
    Customer -- Shop.Order
}
```

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../images/usecase-quickstart-modern-dark.png?v=usecase-actor-balanced-20261104">
  <img src="../images/usecase-quickstart-modern.png?v=usecase-actor-balanced-20261104" alt="Customer actor connected to a use case inside the Shop boundary">
</picture>

[Complete source](../../examples/usecase-quickstart.laml). Declare use cases inside a `system` boundary; connect external actors with `--` and a qualified use-case ID.

## Relationships between use cases and actors

```laml
// Excerpts from examples/usecase-order.laml
semantics {
    // […]
    system Shop {
        // […]
        Order ..> Payment <<include>>
        Discount ..> Order <<extend>>
    }
    // […]
    ReturningCustomer --|> Customer
}
```

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../images/usecase-order-modern-dark.png?v=usecase-actor-balanced-20261104">
  <img src="../images/usecase-order-modern.png?v=usecase-actor-balanced-20261104" alt="Included payment, optional discount and actor generalization">
</picture>

[Complete source](../../examples/usecase-order.laml).

| Relation | Use |
|---|---|
| `Actor -- System.UseCase` | Association. |
| `A ..> B <<include>>` | A includes B. |
| `A ..> B <<extend>>` | A extends B. |
| `Special --|> General` | Generalization of same-kind elements. |

Positioning and styling: [GNE](gne.md). [Full syntax](../language-reference.md#use-case-diagrams).
