# UML 2.5.1 timing notation audit

Reference: OMG UML 2.5.1 §17.11.1–17.11.2, especially Figure 17.28 (discrete lifeline), Figure 17.29 (compact state notation) and Figure 17.30 (multi-lifeline messages). LAML generates a checked **image subset** rather than UML execution semantics.

| UML visual | LAML source | Boundary |
|---|---|---|
| Interaction frame | `interaction: Access` | Enclosing `sd Access`; one frame, no execution model. |
| Lifeline | `lifeline User(of=Customer) : Idle` | Named instance/classifier and initial discrete state. No continuous/general-value lifeline. |
| State or condition timeline | `state User at 2 : WaitCard` | Shared numeric time scale, horizontal levels and vertical changes; strictly increasing times on each lifeline. State text is visible, not a simulated state machine. |
| Event or stimulus label | `event User at 2 : Code` | Visible text associated with an explicit state change on that lifeline, **not** an inter-lifeline UML Message. |
| Time ruler | `tick 0`, `tick 2`, `scale: 105`, `time-unit: ms` | Fixed numeric x coordinates; no implicit unit conversion or temporal retiming for layout. |

Figure 17.28 shows event labels, explicit tick marks and a discrete state timeline. LAML's first subset covers those graphics, including multiple vertically separated timelines, but does **not** claim Figure 17.30's inter-lifeline messages or constraints. UML time observations, time/duration constraints, continuous states, messages and destruction must be implemented as distinct checked notation before admission. Unsafe collision geometry fails explicitly.
