# UML interaction-overview diagrams

Set `type: interaction-overview` and `interaction: Name` in `settings`. LAML renders an `sd Name` interaction frame containing **InteractionUse** (`ref`) frames connected by control flows, with UML initial/final, decision/merge and fork/join symbols. This is an image notation subset, not a flow executor; a `ref` name is visible text, not a resolved source-file reference. See the [alternative example](../examples/interaction-overview-purchase.laml) and [parallel example](../examples/interaction-overview-parallel.laml).

```text
settings {
    type: interaction-overview
    interaction: Checkout
    direction: top-to-bottom
}
semantics {
    initial Start
    interaction-use Validate(ref=CheckAccount)
    final Done
    Start --> Validate
    Validate --> Done
}
layout {
    Start above Validate : gap=95; align=center
    Validate above Done : gap=95; align=center
}
```

`interaction-use Instance(ref=InteractionName)` draws a rectangular `ref` fragment with the referenced interaction name inside. Its own identifier (`Instance`) addresses placement/styling; it is **not** an activity action or an inline sequence diagram. The only other vertices here are `initial`, `final`, `decision`, `merge`, `fork` and `join`. Connect them with unadorned `-->` arrows. Each outgoing decision flow requires a visible `[guard]` or `[else]` label; guards are not evaluated. Else may occur at most once per decision. Other flow labels, object flows, arbitrary actions and UML messages are not accepted. Use `layout { relation[branch] : label=along }` for a named branch if the default guard placement cannot fit. Mandatory relation-end sides can help separate flows into a merge or join; an unsafe route fails rather than being silently reinterpreted.

Exactly one initial, one final and at least one `ref` invocation are required. All vertices must participate in an acyclic connected graph. Decisions must reconverge at a matching merge and forks at a matching join, with properly nested branches. In this first subset every branch reaches its matching join; cycles (including loop interactions), inline interactions, lifeline clauses, time constraints, interruption and arbitrary activity-model nodes are deferred. Required nodes and flows cannot be hidden by LSS; `ref` boxes must retain their square frame, readable name and visible corner tab. `check` validates identity and topology; `build` additionally measures labels and routes and produces SVG, PNG or vector PDF. See the [UML notation audit](uml-interaction-overview-conformance.md).
