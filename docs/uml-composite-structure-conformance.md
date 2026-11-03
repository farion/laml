# UML 2.5.1 composite-structure notation

Reference: OMG UML 2.5.1 §§11.2.3–11.2.4 and 11.3.4. LAML draws a structured classifier with its internal structure and typed ports:

| Drawing | Source |
|---|---|
| Internal-structure compartment | `structure Checkout { ... }` |
| Solid composite part | `part store(of=Store)` |
| Dashed reference role | `role gateway(of=Gateway)` |
| Boundary port and interface contract | `port In(provides=API)` or `port Out(requires=API)` |
| Assembly between sibling roles | `gateway::Backend == store::In` |
| Delegation from owner to contained role | `Checkout::Front => gateway::Public` |

Port contracts use locally declared interfaces. See the [composite-structure guide](composite-structure-diagrams.md) and [checkout example](../examples/composite-checkout.laml).
