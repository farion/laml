# UML communication diagrams

Set `type: communication`. LAML draws a **bounded communication interaction image**: named lifelines connected by explicit links, with directed numbered messages drawn beside those links. It uses its own source form with explicitly numbered messages beside links. The [order example](../examples/communication-order.laml) builds to SVG, PNG and vector PDF.

```text
settings {
    type: communication
    interaction: Checkout
}
semantics {
    object Buyer(of=Customer)
    object Shop(of=Store)
    object Payment(of=Gateway)
    Buyer -- Shop
    Shop -- Payment
    message 1 Buyer --> Shop : placeOrder()
    message 1.1 Shop --> Payment : authorize()
    message 1.2 Payment --> Shop : approved()
    message 2 Shop --> Buyer : confirmation()
}
layout {
    Buyer < Shop : gap=220; align=center
    Shop < Payment : gap=220; align=center
}
```

`interaction: Name` names the enclosing `sd` frame. Each explicitly declared `object Instance(of=Type)` is an underlined, typed lifeline head. An unmarked `A -- B` is a **link**, not a message; only one link per unordered pair of lifelines is accepted. `message N Sender --> Receiver : name(arguments)` draws a small arrow near its label on that link; multiple messages may use the same link and may travel in opposite directions. The arrow in the `message` declaration specifies the sender-to-receiver direction.

The sequence expression `N` is explicit, unique and uses positive dot-separated integers. A nested `1.2` requires a `1` message in this source. Unnumbered, duplicate, malformed or unlinked messages fail. Labels must fit without covering lifelines, other links or message labels; add mandatory `layout` separation when necessary. Normal LSS node/link styling applies; links carrying messages cannot be hidden. `check` validates source and identity, while `build` also validates geometry and renders the image.

See the [UML notation audit](uml-communication-conformance.md) for the checked link and message forms.
