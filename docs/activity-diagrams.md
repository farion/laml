# UML activity diagrams

Set `type: activity`. LAML draws UML 2.5.1 activity notation with actions, object nodes, control and object flows, decisions and forks. The complete order example is [`examples/activity-order.laml`](../examples/activity-order.laml).

## Nodes

- `initial Start`: filled circle. One outgoing control flow, no incoming control flow; at most one initial in this subset.
- `action Inspect`: rounded rectangle. A multiline action body may declare `input order` and `output receipt` on separate lines; these create visible, named input (left) and output (right) squares. An action's control flow connects to the action itself; its object flow connects to the pin as `Inspect::order` or `Inspect::receipt`.
- `objectNode Order`: rectangle showing the object node's name. It may connect only to object flows or note attachments.
- `decision Valid` and `merge Combine`: distinct source declarations and diamond glyphs. Decision has exactly one incoming control flow and at least two guarded outgoing control flows; merge has at least two incoming and exactly one outgoing control flow.
- `fork Parallel` and `join Together`: distinct filled bars perpendicular to the selected diagram flow. Fork has one incoming and at least two outgoing control flows; join has at least two incoming and one outgoing control flow.
- `activityFinal Done` is a bullseye; `flowFinal Rejected` is a circle with an X. Both are terminal and represent distinct UML endings.
- `note Hint "text"` and `Hint .. Inspect` are unadorned dotted annotations, not flows.

## Edges

Use `control A --> B` for a control flow, or `object A::output --> Result` / `object Result --> B::input` for an object flow. Both draw a solid directed arrow: the explicit source keyword and checked endpoints carry the distinction. An object flow may also connect an output pin directly to an input pin. An object arrow cannot connect directly to an action body, and an input pin cannot source an object flow. Decision exits require a literal label such as `control Valid --> Dispatch : [approved]` or `control Valid --> Reject : [else]`; at most one `[else]` is allowed. Guards are text, not computed conditions. Other arrow operators and arbitrary line styles cannot substitute for the UML symbols.

`laml build` validates the drawn control and object flows, their endpoints and their routed geometry.
