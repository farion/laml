# UML timing diagrams: discrete state and condition timelines

Set `type: timing` and `interaction: Name`. LAML draws one or more lifelines in an `sd` interaction frame with **discrete state/condition levels** on a shared numeric horizontal time axis. See the [buildable access example](../examples/timing-access.laml).

```text
settings {
    type: timing
    interaction: Access
    time-unit: ms
}
semantics {
    lifeline User(of=Customer) : Idle
    state User at 2 : WaitCard
    state User at 5 : WaitAccess
    state User at 8 : Idle
    event User at 2 : Code
    event User at 5 : CardOut
    event User at 8 : OK

    lifeline System(of=AccessService) : NoCard
    state System at 3 : HasCard
    state System at 6 : Checking
    state System at 8 : Ready
    event System at 3 : CardInserted
    event System at 8 : AccessGranted
}
layout {
    scale: 105
    lane-gap: 62
    end: 11
    tick 0
    tick 2
    tick 3
    tick 5
    tick 6
    tick 8
    tick 10
}
```

`lifeline Instance(of=Classifier) : InitialState` names an instance and an initial state; `(of=Classifier)` is optional. `state Instance at Time : StateName` changes that lifeline at an explicitly supplied positive decimal time. Times must increase **strictly** per lifeline; revisiting an earlier state is allowed, but repeating the current state is not a transition. State names are displayed next to their horizontal levels. `event Instance at Time : label` labels a declared state change at that time.

`layout` fixes time-to-horizontal-distance with `scale` (pixels per time unit, default 70), spacing between lifeline panels with `lane-gap` (default 62), and the endpoint with `end` (default two units after the last change or tick). The ruler requires `tick 0` and at least one positive tick; additional ticks share the **same scale** as all lifelines. `time-unit` labels the visible time axis. Values too close to draw, overlapping tick/event labels, nonfinite values, missing transitions and an `end` not beyond every change/tick **fail** instead of silently shifting time. Extremely large canvases are rejected. `check` validates declarations and time identity; `build` also validates geometry and exports SVG (selectable embedded text by default), PNG and vector PDF.

LSS imports work. Within a `styling` section, `node[User]` rules may set `color`, `border-color`, `font-size` and `border-width` (one property per line) for the lifeline's names, wave and event text. The compiler preserves the timeline's required shape and visible marks during styling. See the [UML 2.5.1 notation audit](uml-timing-conformance.md) for the discrete-timeline drawing rules.
