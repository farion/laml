# UML 2.5.1 activity notation

Reference: OMG UML 2.5.1 §15.2.4. LAML draws the following activity symbols with checked endpoints and control topology:

| Drawing | Source |
|---|---|
| Rounded action with input/output pins | `action Inspect`, `input order`, `output receipt` |
| Object node | `objectNode Order` |
| Filled initial circle | `initial Start` |
| Activity final / flow final | `activityFinal Done` / `flowFinal Rejected` |
| Decision / merge diamonds | `decision Valid` / `merge Combine` |
| Fork / join bars | `fork Parallel` / `join Together` |
| Solid control / object flows | `control A --> B` / `object A::out --> B::in` |

See the [activity guide](activity-diagrams.md) and [buildable order example](../examples/activity-order.laml).
