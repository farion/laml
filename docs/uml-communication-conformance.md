# UML 2.5.1 communication notation

Reference: OMG UML 2.5.1 §§17.3.4, 17.4.4 and 17.9. LAML draws an enclosing `sd` frame, underlined object heads, links and numbered directed messages:

| Drawing | Source |
|---|---|
| Interaction frame | `interaction: Checkout` |
| Typed object head | `object Buyer(of=Customer)` |
| Link | `Buyer -- Shop` |
| Directed message on a link | `message 1 Buyer --> Shop : placeOrder()` |
| Nested sequence number | `message 1.1 Shop --> Payment : authorize()` |

Message numbers use unique positive dot-separated integers. Nested numbers reference their parent number. See the [communication guide](communication-diagrams.md) and [order example](../examples/communication-order.laml).
