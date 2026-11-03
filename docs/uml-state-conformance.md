# UML 2.5.1 state-machine notation

Reference: OMG UML 2.5.1 §§14.2.4.1 and 14.2.4.3–14.2.4.8. LAML draws these behavior state-machine symbols:

| Drawing | Source |
|---|---|
| Rounded state | `state Ready` |
| Entry/do/exit compartment | `entry / open()`, `do / work()`, `exit / close()` |
| Composite with one visible region | `state Processing { ... }` |
| Filled initial pseudostate | `initial Start` |
| Bullseye final state | `final Done` |
| Guarded choice diamond | `choice Branch` |
| Directed transition | `A --> B : trigger [guard] / effect` |
| Dotted note attachment | `Hint .. Ready` |

The compiler checks pseudostate topology, guarded branches and region-scoped transitions. See the [state guide](state-diagrams.md) and [composite example](../examples/state-composite.laml).
