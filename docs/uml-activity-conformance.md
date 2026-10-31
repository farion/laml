# UML 2.5.1 activity notation boundary

LAML draws a checked subset of UML 2.5.1 activity diagrams (§15.2.4); it does not implement activity execution or token semantics.

| UML concept | Image notation in `type: activity` | Deliberate boundary |
|---|---|---|
| Action | Rounded rectangle with visible name. | Action behavior is not executed. |
| Input/output pin | Named square on left/right action boundary. | Data typing, pin multiplicity and streaming not modeled. |
| Object node | Named rectangle. | No object-value or state model. |
| Initial node | Small filled circle. | At most one initial in this first subset. |
| Activity final / flow final | Bullseye / circled X, separately declared. | Their termination effects are not simulated. |
| Decision / merge | Diamonds with distinct source kinds and checked control topology. | Only literal textual guards; no guard evaluation. |
| Fork / join | Filled bars with distinct source kinds and checked control topology. | Control-flow forks/joins only; no object-token splitting or synchronization. |
| Control flow / object flow | Solid directed arrows with explicit source keywords; object edges attach only to object nodes and direction-checked pins. | No fabricated line-style distinction or implicit conversion between flow kinds. |

Particularly, drawing an object-flow arrow into the center of an action while hiding its input/output identity would misrepresent this checked subset. Unrecognized relationship operators, mixed-up pin directions, missing decision guards and incorrect pseudonode topology are errors rather than downgraded drawings. The [activity guide](activity-diagrams.md) describes the language syntax and example.
