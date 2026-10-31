# UML 2.5.1 communication notation audit

Reference: OMG UML 2.5.1 §§17.3.4 (lifelines), 17.4.4 (message notation), 17.9 and 17.9.1 (communication diagrams). This is a **checked image subset**, not execution or complete UML abstract syntax.

| UML visual | LAML notation | Checked boundary |
|---|---|---|
| Interaction frame | `interaction: Checkout`, shown as `sd Checkout` on an enclosing frame. | One named interaction; frame contains all declared lifelines, links and message labels. |
| Lifeline head | `object Buyer(of=Customer)` with underlined name/type. | No vertical time axis, inline decomposition or nested object body. |
| Link | `Buyer -- Shop` plain undirected line. | Exactly one for every communicating pair; a dashed or directed dependency is not a link. |
| Directed message | `message 1 Buyer --> Shop : placeOrder()`; small arrow near the number and label on the declared link. | Arrow direction follows message sender/receiver, independently of source link order. Message sort is not inferred. |
| Sequence expression | `1`, `1.1`, `1.2`, `2` explicitly written. | Unique positive integer terms; a nested term needs its parent. Spatial placement is not time order. |

UML also defines concurrently named sequence terms, recurrences and guards in the sequence expression, as well as further message kinds. This first subset rejects them. Link geometry and message labels must fit; no message arrow is silently converted to a UML association or dependency, and an unsafe rendering fails.
