# Activity diagrams

## Basic example

```laml
settings {
    type: activity
}
semantics {
    initial Start
    objectNode Request
    action Review {
        input request
    }
    activityFinal Done

    object Request --> Review::request
    control Start --> Review
    control Review --> Done
}
```

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../images/activity-basic-modern-dark.png?v=activity-control-spine-20261104">
  <img src="../images/activity-basic-modern.png?v=activity-control-spine-20261104" alt="Request enters an action pin alongside the control flow">
</picture>

[Complete source for this image](../../examples/activity-basic.laml). `object` carries a request to the action's named input pin; `control` connects the start, action and activity final. The default layout keeps the control flow left-to-right and places a lone input object beneath its action.

## Decisions, parallel work and completion

```laml
// Excerpts from examples/activity-order.laml
semantics {
    // […]
    control Valid --> Reject : [invalid]
    @approved control Valid --> Parallel : [else]
    control Parallel --> Reserve
    control Parallel --> Authorize
    control Reserve --> Together
    control Authorize --> Together
    // […]
}
styling {
    node[Parallel] {
        length: 170
    }
}
```

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../images/activity-order-modern-dark.png?v=activity-fork-length-20261104">
  <img src="../images/activity-order-modern.png?v=activity-fork-length-20261104" alt="Order activity with guarded decision and extended fork/join">
</picture>

[Complete source](../../examples/activity-order.laml). A `decision` uses guarded outgoing paths; `fork` and `join` split and rejoin parallel paths. `length` extends the fork along its long axis so both outgoing arrows can run horizontally. `flowFinal` ends one path, `activityFinal` ends the activity.

## More data flows

```laml
// Excerpts from examples/activity-order.laml
object Reserve::reservation --> Reservation
object Reservation --> Dispatch::reservation
object Authorize::approval --> Approval
object Approval --> Dispatch::approval
```

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../images/activity-order-modern-dark.png?v=activity-fork-length-20261104">
  <img src="../images/activity-order-modern.png?v=activity-fork-length-20261104" alt="Named action pins and object nodes in the order activity">
</picture>

[Complete source](../../examples/activity-order.laml). `Action::pin` addresses an input or output pin. See [GNE](gne.md) for common placement/styling and the [syntax reference](../language-reference.md#activity-diagrams) for vertex and flow forms.
