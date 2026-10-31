# UML 2.5.1 state-machine notation audit

This audit concerns **behavior state-machine diagrams as images**. It is not conformance to UML's abstract syntax, a run-to-completion execution engine, or a protocol state machine. Reference: OMG *Unified Modeling Language* 2.5.1 §§14.2.4.1, 14.2.4.3–14.2.4.8.

| Notation | LAML source and output | Boundary |
|---|---|---|
| Simple state | `state Ready`: measured rounded rectangle with visible name. | No anonymous-state sharing, protocol-state invariants or hidden automaton. |
| State behavior | `entry / open()`, `do / work()`, `exit / close()` in a state body: visible text compartment. | One of each; actions are not executed or type-checked. |
| Composite state | `state Processing { ... }` encloses a single region with its own scoped vertices and edges. | No orthogonal multi-region decomposition, submachine reference or hidden-decomposition icon. |
| Initial pseudostate | `initial Start`: filled circle without visible name; exactly one outgoing unlabeled transition and no incoming transition. | At most one per visible region; creation-event-labeled initial transitions omitted. |
| Final state | `final Done`: circle containing a filled smaller circle. | Outgoing transitions rejected; completion is not simulated. |
| Choice pseudostate | `choice Branch`: diamond with guarded outgoing branches and at most one `[else]`. | No expression evaluation, junction merging or dynamic decision execution. |
| Directed transition | `A --> B : trigger [guard] / effect`: solid directed arc and literal text. | Other graph arrows, unsupported edge adornments and cross-region edges fail. Trigger kind and guard truth are not inferred. |
| Annotation | `note Hint "..."` followed by `Hint .. Ready`: dotted note line. | Not an executable transition. |

UML §§14.2.4.5–14.2.4.6 additionally define history, entry/exit, terminate, junction, fork and join symbols; these are deliberately rejected until their topology and notation are independently checked. GNE supplies collision-checked layout and routing, not UML behavior semantics.
