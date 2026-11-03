# UML 2.5.1 sequence notation

Reference: OMG UML 2.5.1 clauses 8 and 17. LAML draws these sequence symbols with named, measured event positions:

| Drawing | Source |
|---|---|
| Lifeline head, selector and type | `API(selector="u"; type="Service")` |
| Execution bars | `API+`, `API-`, `work: API+ : "Validate"` |
| Synchronous, asynchronous and reply arrows | `A -> B`, `A ->> B`, `A <-- B` |
| Creation and destruction | `create(kind=create): A -> B+`, `A -> B!` |
| Found and lost endpoints | `? -> API`, `API -> ?` |
| Combined fragments and operands | `alt`, `par`, `opt`, `loop`, `break`, `strict`, `seq`, `critical`, `neg`, `assert`, `ignore`, `consider` |
| Interaction-use frame and gates | `ref "Lookup" { ... }`, `gate[done]` |
| State invariant and action | `at[request.receive] : invariant="ready"`, `action="Read"` |
| Continuation and coregion | `continuation`, `coregion API { ... }` |
| General ordering and time/duration annotations | `order[first.receive,second.send]`, `at[...]`, `between[...]` |
| Inline part decomposition | `decompose API "Internals" { ... }` |

See the [sequence guide](sequence-diagrams.md) and [complete examples](../examples/sequence-basic.laml).
