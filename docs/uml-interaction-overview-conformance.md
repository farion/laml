# UML 2.5.1 interaction-overview notation

Reference: OMG UML 2.5.1 §§17.2.4, 17.7.4.1 and 17.10.1. LAML draws interaction uses connected by activity-style control flow:

| Drawing | Source |
|---|---|
| Enclosing `sd` frame | `interaction: Checkout` |
| Rectangular `ref` interaction use | `interaction-use Login(ref=EstablishAccess)` |
| Control vertices | `initial`, `final`, `decision`, `merge`, `fork`, `join` |
| Solid directed flow | `A --> B` |
| Guarded decision exit | `Decision --> Next : [guard]` or `[else]` |

The compiler checks one entry and exit and matching decision/merge and fork/join structure. See the [interaction-overview guide](interaction-overview-diagrams.md) and [purchase example](../examples/interaction-overview-purchase.laml).
