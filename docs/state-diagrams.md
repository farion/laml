# UML state diagrams

Set `type: state` in `settings`. LAML draws UML 2.5.1 behavior state-machine notation with states, pseudostates, transitions and visible labels. Geometry, scoped references, mandatory layout, LSS styling and SVG/PNG/vector-PDF exports reuse the graph engine.

## States, pseudostates and transitions

```text
settings {
    type: state
}
semantics {
    initial Start
    state Idle
    choice Check
    state Active {
        entry / startSession()
        do / awaitWork()
        exit / stopSession()
    }
    final Done
    Start --> Idle
    @submit Idle --> Check : submit / validate()
    @accept Check --> Active : [valid]
    @retry Check --> Idle : [else]
    @stop Active --> Done : stop
}
```

`state Name` is a rounded rectangle. `initial Name` is a filled small circle, `final Name` is a bullseye and `choice Name` is an empty diamond. Names of pseudostates are stable **source IDs**, not drawn text. `A --> B` is a directed UML transition; other graph arrow operators cannot stand in for it. Its optional label uses `trigger [guard] / effect` in that order, with any omitted parts left out. The strings are displayed verbatim; LAML checks delimiters and pseudostate rules. The initial transition must be unlabeled. A choice needs an incoming transition and at least two guarded exits; one `[else]` is allowed. A final state has no outgoing transitions. `Note .. State` remains a dotted, unadorned annotation, not a transition. See the [basic example](../examples/state-basic.laml).

A state body can contain at most one each of `entry / action`, `do / action`, and `exit / action`. These appear in an internal-behavior compartment. An unlabeled `State --> State` depicts a completion transition. The [self-transition example](../examples/state-loop.laml) shows guarded transitions, effects and an exterior loop.

## Composite state: one visible region

A `state Outer { ... }` may contain `state`, `initial`, `final`, `choice`, and notes. Nested names are scoped (`Outer.Running`), and transitions written inside it resolve local names. Each displayed region has at most one initial pseudostate with exactly one outgoing transition; it may have a final state. Transitions within the composite connect vertices in the same immediate region. To depict a completion transition from the composite to an outer state, connect the **composite state itself** to the outer target; a direct edge from an inner vertex to an unrelated outer region is rejected. The [composite example](../examples/state-composite.laml) shows inner initial/final vertices and an outer completion arrow. A fixed-size composite too small for its children fails rather than clipping them.

## Drawing checks

Each composite state has one visible region. Duplicate initials, invalid initial exits, outgoing final transitions, unguarded choice exits and cross-region transitions produce source errors. See the [UML 2.5.1 notation audit](uml-state-conformance.md) for the checked notation.
