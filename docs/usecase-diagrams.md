# UML use-case diagrams

Set `type: usecase`. LAML draws UML 2.5.1 use-case notation with actors, a system boundary and named use cases. The [order example](../examples/usecase-order.laml) builds as SVG, PNG and vector PDF.

```text
settings {
    type: usecase
}
semantics {
    actor Customer
    actor ReturningCustomer(label="Returning\nCustomer")
    system Shop {
        usecase Order(label="Place order")
        usecase Payment(label="Authorize payment")
        usecase Discount(label="Apply discount")
        Order ..> Payment <<include>>
        Discount ..> Order <<extend>>
    }
    Customer -- Shop.Order
    ReturningCustomer --|> Customer
}
```

`system Name { ... }` is the labeled system boundary and contains named `usecase` ellipses. Modern themes give the subject frame softly rounded corners; plain themes retain square corners. The actor figure has a fixed silhouette independent of its caption width. Relations attach to an invisible ellipse around the figure, not to its arm or caption. Automatic placement aligns the visible figures of generalized actors and prefers matching an actor's midpoint to its connected use case. If another use case blocks a lateral association, routing can leave the actor's side and bend around the obstruction instead of leaving near its head. Actors are external stick figures and cannot be nested inside that boundary. Child IDs are scoped (`Shop.Order`); an explicit `@id` remains diagram-wide. This **source scope expresses visible placement**, not UML ownership: the subject rectangle does not itself imply ownership of the depicted use cases. A diagram has one system boundary with at least one use case. Labels can be set independently from IDs using `(label="Visible text")`. Notes attach through unadorned dotted lines.

Relationships are checked by endpoint kind, direction and UML marker:

- `Actor -- UseCase`: plain solid association; either source order is allowed. The system boundary itself is not an association endpoint.
- `Base ..> Included <<include>>`: dashed directed dependency **from including to included** use case.
- `Extension ..> Base <<extend>>`: dashed directed dependency **from extension to extended** use case. The arrow identifies the extending and extended use cases.
- `Child --|> Parent`: solid open-triangle generalization between two actors or between two use cases in the same system. A generalization cycle fails.

Graph placement direction and UML relationship direction are separate. The compiler checks endpoint kinds, arrows and system boundaries. See the [UML notation audit](uml-usecase-conformance.md).
