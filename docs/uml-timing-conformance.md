# UML 2.5.1 timing notation

Reference: OMG UML 2.5.1 §17.11.1–17.11.2, especially Figure 17.28. LAML draws discrete lifelines on a shared numeric time axis:

| Drawing | Source |
|---|---|
| Enclosing `sd` frame | `interaction: Access` |
| Lifeline and initial state | `lifeline User(of=Customer) : Idle` |
| State or condition change | `state User at 2 : WaitCard` |
| Event label at a declared change | `event User at 2 : Code` |
| Numeric ruler | `tick 0`, `tick 2`, `scale: 105`, `time-unit: ms` |

Time coordinates are explicit and shared across lifelines. See the [timing guide](timing-diagrams.md) and [access example](../examples/timing-access.laml).
