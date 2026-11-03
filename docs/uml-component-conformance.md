# UML 2.5.1 component notation

Reference: OMG UML 2.5.1 §§10.4.4, 11.2.4, 11.3 and 11.6.4–11.6.5. LAML draws these component symbols:

| Drawing | Source |
|---|---|
| Component classifier and `«component»` keyword | `component Name` |
| Dashed component dependency | `A ..> B` |
| Interface realization or requirement | `Component ..|> Interface` / `Component ..> Interface` |
| Compact provided/required interface | Named relation styled with `interface-notation: lollipop` or `socket` |
| Typed boundary port | `port Name(provides=I)` or `port Name(requires=I)` |
| Contained part | `part p(of=Type)` |
| Assembly between sibling ports | `partA::provided == partB::required` |
| Delegation from owner to part | `Owner::boundary => part::port` |

Assembly and delegation connect visible port squares; dependencies connect component definitions. See the [component guide](component-diagrams.md) and [internal wiring example](../examples/component-internal.laml).
