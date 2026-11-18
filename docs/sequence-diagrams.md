# Sequence diagrams

Set `type: sequence` in `settings`. A `.laml` file separates `semantics` (participants and events), `layout` (mandatory placement), and `styling` (LSS). Build with `laml build examples/sequence-basic.laml output.svg`, replacing `.svg` with `.png` or `.pdf` as needed. `laml check` checks declarations and references; `laml build` also validates measured geometry.

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

A participant declaration draws its head and dashed lifeline. `API+` opens an execution bar; `API-` closes it. Messages arriving at or leaving an active execution attach to its visible edge on the side of the other endpoint. The short `<--` reply resolves the last unambiguous open call; write `response: Client <-- API : 200 OK` when its endpoints need to be explicit. The `layout` rule keeps full participant heads at least 80 pixels apart. Message order provides a stable row tie-break for placement. The source of [sequence-basic.laml](../examples/sequence-basic.laml) and its [rendered image](images/sequence-basic-modern.png?v=41865e08f9) provide a shorter variation.

## Messages and lifetimes

| Source | Drawing |
|---|---|
| `A -> B : call` / `B <- A : call` | Filled synchronous arrow toward B. |
| `A ->> B : signal` / `B <<- A : signal` | Open asynchronous arrow toward B. |
| `A <-- B : result` / `<-- : result` | Dashed reply, explicit or unambiguously inferred. |
| `A -> A : retry` | Self-message. |
| `A+ -> B+ : call` | Start A's activation at send and B's activation at receive. |
| `A- -> B- : done` | End the most recent activation at each endpoint. |
| `? -> A` / `A -> ?` | Found/lost message with an unknown endpoint. |
| `A -> B+ : create` | First incoming `+` can introduce B, with an execution bar. |
| `create(kind=create): A -> B+ : create` | Explicit dashed/open creation message. |
| `A -> B! : delete` / `B! <- A : delete` | Destruction X and end of B's lifeline. |

Each message endpoint accepts its own `+` or `-`, independently of the other
endpoint and the arrow direction. For example, `A- -> B+` ends A's activation
and starts B's activation on the same message. On self-messages, source markers
refer to the send leg and target markers to the receive leg.

A standalone `B+` starts an execution bar; `B-` closes the most recent open bar. Nested bars can have participant-local IDs and visible labels, e.g. `work: API+ : "Validate"`; `node[API] activation[work]` styles that bar. Lifeline births and deaths can occur inside an alternative operand: the same ID can have separate segments in different operands. References to a participant after its destruction are errors. See [created](../examples/sequence-created.laml), [execution](../examples/sequence-executions.laml) and [alternative-births](../examples/sequence-alternative-births.laml).

## Align activation edges

Use `+` for activation tops and `-` for activation bottoms in equal-height layout:

```text
settings {
    type: sequence
}
semantics {
    A
    B
    first: A+
    second: B+
    B-
    A-
}
layout {
    A.first+ ? B.second+ : align=center-y
    A.first- ? B.second- : align=center-y
}
```

`A+` or `A-` alone identifies an edge when A has exactly one activation; use
`A.activationName` when there are several. A message name can identify the
activation started by that message. Top alignment extends toward the earlier
row; bottom alignment toward the later row. Attached messages move with their
activation boundary. Rules that reverse events on a lifeline or cross fragment
boundaries fail rather than silently change the interaction.

Drag an activation edge near another matching edge to snap them flush. The editor
adds `reference=second`, so the dragged edge follows the target's existing height
in either direction rather than always extending the bars. For example, the layout
entry `A.first- ? B.second- : align=center-y; reference=second` keeps B's end fixed.
The editor adds stable names where needed and writes the rule in one undo step.
Moving an open lower edge into alignment also creates its explicit close.

An activation edge can also dock directly onto a message endpoint. For a self-call,
its upper send leg and lower receive leg are separate targets: resizing can switch
between them even after docking. An open lower edge can acquire an explicit close;
the innermost activation must end before its enclosing activation can end.

### Align a message occurrence with an activation edge

A message's `.send` or `.receive` occurrence can share the height of an existing
activation's `+` (start) or `-` (finish) edge:

```text
settings {
    type: sequence
}
semantics {
    Client
    Service
    work: Service+ : "Work"
    request: Client -> Service
    Service-
}
layout {
    request.receive ? Service.work+ : align=center-y; reference=second
}
```

`reference=second` keeps the second operand stationary. Without that attribute,
the activation edge is the reference, including when written first. The rule
changes geometry, not the activation's declaration, local ID, label or existing
message binding. Source event order remains authored; sharing a height is not an
execution simulation. An unconstrained message end can be placed automatically
to keep sending before receiving. Explicitly constrained ends are not moved to
resolve a conflict.

Constrain `.send` and `.receive` independently to draw a message between different
heights; use the same reference for both to require a horizontal ordinary message.
Self-messages retain distinct sending and receiving heights. Lower-edge references
require a closed activation. Unknown or ambiguous references, cross-operand rules,
conflicting/cyclic constraints, upward messages and invalid nested/lifetime
placements are diagnosed rather than silently overriding authored intent.

Self-message loops are routed together: a contained call uses an inner corridor
and an enclosing call stays outside it. Automatic lane spacing and canvas/frame
bounds account for these corridors and their labels. Unavoidable message crossings
use bridges or compact underpass gaps. Export and editor picking use the same
compiler paths; drag previews reuse the compiler's self-loop extent.

The editor uses these rules when docking message endpoints, including at named,
labelled or already message-bound activation edges. It adds stable names only
where an address is required and retains existing declarations and comments.
Dragging to a gap releases that endpoint's height binding. Moving a whole message
releases its layout-only endpoint bindings; semantic `+`/`-` markers remain part
of the message. Resizing an activation releases only the edited edge's bindings.

## Align destruction markers

Standalone destruction occurrences can share a horizontal row through an explicit
layout rule:

```text
settings {
    type: sequence
}
semantics {
    Client
    Worker
    Client!
    Worker!
}
layout {
    Client! ? Worker! : align=center-y
}
```

The rule aligns both X markers and their lifeline ends at the later row. Open
activations terminated by those destructions extend to that row as well. Source
event order stays unchanged; this is visual alignment, not a claim of simultaneous
execution. Connected alignment rules form one group. Each reference must identify
one standalone destruction; ambiguous reused lifetimes, message-bound destructions
and rules crossing fragment or operand boundaries are rejected.

In the editor, hover a lifeline end to reveal a ghost X and click to add a
standalone destruction. Drag an actual X to a valid event gap or another X's row;
horizontal guides show the available positions. Moving away from an existing
alignment detaches only the moved member. Right-click the X to delete the
destruction. Escape or dropping away from a valid target cancels the gesture.
Source, preview and exports use the same authored rules.

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

`covers` fixes a named frame's **visible** horizontal scope, including every lifeline between its endpoints; invalid coverage fails. `alt` uses `else`, `par` uses `and`. Supported operators also include `opt`, `loop`, `break`, `strict`, `seq`, `critical`, `neg`, `assert`, `ignore`, and `consider`. Guards and loop bounds appear literally in the image. For example, `loop(min=0; max=*; guard="pending") {` shows bounds and a guard. `ignore "request,response" {` shows a curly-braced message list. Consecutive operator words may share a descriptor (e.g. `strict alt "ready" {`). See [fragments](../examples/sequence-fragments.laml) and [frame coverage](../examples/sequence-frame-coverage.laml).

A `ref` draws an interaction-use frame with the supplied visible label. A named `gate[id]` sits on its enclosing frame boundary. Two opposite-direction messages using the same gate name inside one frame meet at **one point**; using the name twice in the same direction is an error. A gate can join an outside participant to a message inside a scoped frame. The frame and its gates are defined in the diagram source. Fragment operands must agree on open executions and lifeline lifetimes at the join; LAML reports divergent states instead of inventing closures.

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

`at[...]` also supports `state`, `time-constraint`, and `time-observation`; `between[...]` supports duration observation or constraint. These annotations appear as visible text and shapes. Overlapping message labels on independently ordered rows create additional vertical space; frame titles, notes and continuations also reserve space against message labels. Time observations try alternate sides/heights when message text is in the way. Anchored annotation text and its visible connector avoid measured message labels, notes and frame captions. Duration brackets can move to the other side of their anchors, and note links try alternate paths around annotations. If no safe position exists, LAML names the affected anchor. `order[...]` adds a visible general-ordering connection and rejects contradictory explicit cycles. A named execution's explicit `.start` and `.finish` can also be anchors: `execution(API,work).start`. See [annotations](../examples/sequence-annotations.laml) and [order](../examples/sequence-order.laml).

## Notes, continuations, coregions and decomposition

Define a note in `semantics` and **place it in `layout`**: `hint: note "Retry once"` with `hint over API : gap=16` (centered in its own event row), `hint over request.receive : gap=16` (linked to that message end), or `hint right of API : gap=12` (beside the lifeline, without a connector; here `gap` is horizontal). Continuations similarly have a named declaration and a mandatory `spans` rule. A `coregion API { request.receive ... }` marks occurrences on one lifeline that can be reordered with an explicit `above` rule; a positive `gap` is a minimum. Conflicting order or overlap with a later occurrence on that lifeline fails. If send and receive occurrences have different heights, the message has an orthogonal bend by default and retains both event coordinates; `allow-diagonal: true` in `settings` restores a single sloping segment. See [notes](../examples/sequence-notes.laml), [continuations](../examples/sequence-continuation.laml) and [coregion](../examples/sequence-coregion.laml).

`decompose API "Internals" { ... }` draws child lifelines and messages **inline in the same source**. Root layout targets them through qualified IDs such as `API.Cache`. Styles can select context without propagating classes: `node[API] node[Cache]` targets that head, while `node[API] relation[query]` targets its message. See [decomposition](../examples/sequence-decomposition.laml).

## Styling and exports

Graph-style `node` selectors style sequence heads, and `relation` selectors style messages. Sequence-only targets include `lifeline`, `activation`, `fragment`, `note`, `continuation`, `coregion`, `annotation`, `gate`, and `ordering`; named and descendant selectors target individual shapes, for example `fragment[checkout]`, `gate[done]`, `node[API] activation[work]`. Use `node.critical lifeline` to style a lifeline selected by its participant's class. All SVG, PNG and vector-PDF outputs use the same layout; LSS can change sizes before it is solved. See [styled](../examples/sequence-styled.laml), the [language reference](language-reference.md#sequence-diagrams), and the [UML 2.5.1 notation audit](uml-sequence-conformance.md) for the supported drawable notation and explicit boundaries.
