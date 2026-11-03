# UML 2.5.1 use-case notation

Reference: OMG UML 2.5.1 chapter 18. LAML draws actors, system boundaries, use cases and their checked relationships:

| Drawing | Source |
|---|---|
| Actor stick figure | `actor Customer` |
| System boundary | `system Shop { ... }` |
| Use-case ellipse | `usecase Order(label="Place order")` |
| Actor–use-case association | `Customer -- Shop.Order` |
| Include dependency | `Order ..> Payment <<include>>` |
| Extend dependency | `Discount ..> Order <<extend>>` |
| Actor or use-case generalization | `Special --|> General` |
| Dotted note attachment | `Hint .. Shop.Order` |

The compiler checks endpoint kinds, direction and system scope. See the [use-case guide](usecase-diagrams.md) and [order example](../examples/usecase-order.laml).
