# UML 2.5.1 interaction-overview notation audit

Reference: OMG UML 2.5.1 §§17.2.4 (interaction frame), 17.7.4.1 (InteractionUse), 17.10.1 (interaction-overview diagram). LAML generates a checked image subset, not UML execution or an abstract syntax model.

| UML graphic | LAML declaration | Boundary |
|---|---|---|
| Interaction frame | `interaction: Checkout` → enclosing `sd Checkout` frame | One interaction name, no lifeline clause. |
| InteractionUse | `interaction-use Login(ref=EstablishAccess)` → rectangular `ref` fragment containing `EstablishAccess` | No separate interaction definition resolution, argument binding, inline interaction or executable call behavior. A generic action box is never substituted. |
| Activity-style control vertices | `initial`, `final`, `decision`, `merge`, `fork`, `join` | Exactly one entry and exit. Fork/decision and join/merge must match and nest; no cycles in this initial subset. |
| Control flow | `A --> B` | Unadorned solid directed flow only. Decision exits carry `[guard]` or `[else]`; guards are displayed, never evaluated. |

The standard permits inline Interactions, loop cycles and richer InteractionUses, and allows lifeline clauses in the interaction frame. They are intentionally rejected here. Hidden required topology, missing joins, crossing branch nesting and unsafe geometry fail rather than drawing a misleading Activity or Communication Diagram.
