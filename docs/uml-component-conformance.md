# UML 2.5.1 component-notation audit

This is a **notation** audit, not a claim of UML abstract-syntax or execution conformance. The reference is OMG *Unified Modeling Language* 2.5.1, §§10.4.4 (interfaces), 11.2.4 (structured-classifier connectors), 11.3 (ports) and 11.6.4–11.6.5 (components and examples).

| Notation | LAML source and visible output | Boundary |
|---|---|---|
| Component classifier | `component Name` displays `«component»` above its name and the UML corner component glyph in a measured classifier rectangle. | A top-edge port suppresses the optional glyph to avoid overlap; the keyword remains visible. |
| Component dependency | `A ..> B` displays a dashed dependency with open arrow. | Does not imply assembly or direct runtime service wiring. |
| Provided/required interface box | `Component ..|> Interface` is a dashed realization; `Component ..> Interface` is a dashed dependency. | Port roles are not inferred from an unrelated dependency. |
| Compact provided/required interface | Named relation styled with `interface-notation: lollipop` or `socket`; solid stalk and labeled ball or socket replace an otherwise unused empty interface box. | No member-bearing/multiply-used/placed interface box may disappear. Socket is component-only. |
| Simple ports | `port Name(provides=I)` or `port Name(requires=I)` places an addressable, visibly labeled port on the actual classifier boundary. | One declared interface per port; no operation-signature/type compatibility inference. |
| Part/inner structure | `part p(of=Type)` draws a named contained part; its port names and contracts are copied from local `Type`. | Parts must belong directly to the containing component; no nested structured-classifier metamodel. |
| Assembly | `partA::provided == partB::required` is a solid connector between opposite-role, same-interface ports on distinct parts of the same component. | Uses the always-supported port-square connector notation (§11.2.4); optional ball-and-socket assembly glyph and n-ary channeled notation omitted. |
| Delegation | `Owner::boundary => part::port` is a solid connector between corresponding same-role ports in the same ownership context. | Not inferred from generic node-to-node edges. |
| Annotation and container | Notes, packages, labels, scoped references, hard layout constraints, LSS and exports use the graph engine. | Unsupported markers or unsafe geometry fail instead of silently falling back. |

**Distinction preserved:** §11.6.5 depicts dependencies between component *definitions* in an overview. §11.2.4 depicts assembly/delegation between ports on *parts* in an internal structure. One cannot be silently substituted for the other. This image subset does not calculate provided/required interfaces from full UML classifier realization properties or execute connectors.
