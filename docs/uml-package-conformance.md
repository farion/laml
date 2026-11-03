# UML 2.5.1 package notation

Reference: OMG UML 2.5.1 §§7.4.4.3 and 12.2.4. LAML draws tabbed packages and directed, labeled package relationships:

| Drawing | Source |
|---|---|
| Leaf or nested package | `package A` / `package A { package B }` |
| Unmarked dependency | `A ..> B` |
| Public package import | `A ..> B <<import>>` |
| Private package access | `A ..> B <<access>>` |
| Package merge | `A ..> B <<merge>>` |
| Dotted note attachment | `Hint .. A` |

Arrows point from the receiving or importing package to the referenced package. See the [package guide](package-diagrams.md) and [architecture example](../examples/package-architecture.laml).
