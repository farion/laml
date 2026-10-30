# Sequence diagrams: working guide

LAML draws **sequence diagrams**, not executable UML models. Set `type: sequence` in `settings`; without it the graph-diagram grammar applies. A `.laml` file still separates `semantics` (visible participants and events), `layout` (mandatory placement), and `styling` (LSS). Build with `laml build examples/sequence-basic.laml output.svg`, replacing `.svg` with `.png` or `.pdf` as needed. `laml check` checks declarations and references but does **not** validate all rendered geometry: use `build` before relying on an image.

## A complete starting example

```text
settings {
    type: sequence
}
semantics {
    Client(label="Browser")
    API(label="Orders API")

    request: Client -> API+ : GET /orders
    at[request.send] : time-observation="t0"
    response: Client <-- API : 200 OK
    API-
}
layout {
    Client < API : gap=80
}
styling {
    node[API] {
        fill: "#e4f1ff"
    }
    relation[request] {
        color: "#245080"
    }
}
```

A participant declaration draws its head and dashed lifeline; it does not open an execution bar. `API+` opens a bar; `API-` closes it. The short `<--` reply resolves the last unambiguous open call; write `response: Client <-- API : 200 OK` when its endpoints need to be explicit. The `layout` rule keeps full participant heads at least 80 pixels apart. Message order provides a stable row tie-break; it does **not** assert that independent events on different lifelines have a temporal order. The source of [sequence-basic.laml](../examples/sequence-basic.laml) and its [rendered image](images/sequence-basic.png) provide a shorter variation.

## Messages and lifetimes

| Source | Drawing |
|---|---|
| `A -> B : call` / `B <- A : call` | Filled synchronous arrow toward B. |
| `A ->> B : signal` / `B <<- A : signal` | Open asynchronous arrow toward B. |
| `A <-- B : result` / `<-- : result` | Dashed reply, explicit or unambiguously inferred. |
| `A -> A : retry` | Self-message. |
| `? -> A` / `A -> ?` | Found/lost message with an unknown endpoint. |
| `A -> B+ : create` | First incoming `+` can introduce B, with an execution bar. |
| `create(kind=create): A -> B+ : create` | Explicit dashed/open creation message. |
| `A -> B! : delete` / `B! <- A : delete` | Destruction X and end of B's lifeline. |

A standalone `B+` starts an execution bar without a message; `B-` closes the most recent open bar. Nested bars can have participant-local IDs and visible labels, e.g. `work: API+ : "Validate"`; `node[API] activation[work]` styles that bar. Lifeline births and deaths can occur inside an alternative operand: the same ID can have separate segments in different operands. References to a participant after its destruction are errors. See [created](../examples/sequence-created.laml), [execution](../examples/sequence-executions.laml) and [alternative-births](../examples/sequence-alternative-births.laml).

## Fragments and interaction uses

```text
settings {
    type: sequence
}
semantics {
    Client
    API
    checkout: alt "ready" {
        Client -> API : submit
    }
    else "busy" {
        Client ->> API : retry later
    }
    lookup: ref "Lookup(item): result" {
        API -> gate[done] : complete
    }
}
layout {
    checkout covers Client,API
    lookup covers API
}
```

`covers` fixes a named frame's **visible** horizontal scope, including every lifeline between its endpoints; invalid coverage fails. `alt` uses `else`, `par` uses `and`. Supported operators also include `opt`, `loop`, `break`, `strict`, `seq`, `critical`, `neg`, `assert`, `ignore`, and `consider`. Guards and loop bounds appear literally; they are never executed. For example, `loop(min=0; max=*; guard="pending") {` shows bounds and a guard. `ignore "request,response" {` shows a curly-braced message list. Consecutive operator words may share a descriptor (e.g. `strict alt "ready" {`). See [fragments](../examples/sequence-fragments.laml) and [frame coverage](../examples/sequence-frame-coverage.laml).

A `ref` draws an interaction-use frame; its label is not a file import. A named `gate[id]` sits on its enclosing frame boundary. Two opposite-direction messages using the same gate name inside one frame meet at **one point**; using the name twice in the same direction is an error. A gate can join an outside participant to a message inside a scoped frame. No implicit matching to an external UML interaction occurs. Fragment operands must agree on open executions and lifeline lifetimes at the join; LAML reports divergent states instead of inventing closures. The outer `sd` interaction frame is not part of v1.2.

## Anchors, annotations and order

Name a message to refer to its `name.send` and `name.receive` ends:

```text
request: Client -> API : query
at[request.send] : time-observation="t0"
at[request.receive] : invariant="ready"
at[request.receive] : action="Read"
response: Client <-- API : result
between[request.send,response.receive] : duration-constraint="<= 200 ms"
order[request.receive,response.send]
```

`at[...]` also supports `state`, `time-constraint`, and `time-observation`; `between[...]` supports duration observation or constraint. These are visible text/shapes, not evaluated conditions. `order[...]` adds a visible general-ordering connection and rejects contradictory explicit cycles. A named execution's explicit `.start` and `.finish` can also be anchors: `execution(API,work).start`. See [annotations](../examples/sequence-annotations.laml) and [order](../examples/sequence-order.laml).

## Notes, continuations, coregions and decomposition

Define a note in `semantics` and **place it in `layout`**: `hint: note "Retry once"` with `hint over API : gap=16` or `hint over request.receive : gap=16`. Continuations similarly have a named declaration and a mandatory `spans` rule. A `coregion API { request.receive ... }` marks occurrences on one lifeline that can be reordered with an explicit `above` rule; a positive `gap` is a minimum. Conflicting order or overlap with a later occurrence on that lifeline fails. See [notes](../examples/sequence-notes.laml), [continuations](../examples/sequence-continuation.laml) and [coregion](../examples/sequence-coregion.laml).

`decompose API "Internals" { ... }` draws child lifelines and messages **inline in the same source**. Root layout targets them through qualified IDs such as `API.Cache`. Styles can select context without propagating classes: `node[API] node[Cache]` targets that head, while `node[API] relation[query]` targets its message. See [decomposition](../examples/sequence-decomposition.laml).

## Styling and exports

Graph-style `node` selectors style sequence heads, and `relation` selectors style messages. Sequence-only targets include `lifeline`, `activation`, `fragment`, `note`, `continuation`, `coregion`, `annotation`, `gate`, and `ordering`; named and descendant selectors target individual shapes, for example `fragment[checkout]`, `gate[done]`, `node[API] activation[work]`. A participant's class does **not** automatically become a lifeline class: use `node.critical lifeline`. All SVG, PNG and vector-PDF outputs use the same layout; LSS can change sizes before it is solved. See [styled](../examples/sequence-styled.laml), the [language reference](language-reference.md#sequence-diagrams), and the [UML 2.5.1 notation audit](uml-sequence-conformance.md) for the supported drawable notation and explicit boundaries.
