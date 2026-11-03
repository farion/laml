# UML 2.5.1 class and object notation

Reference: OMG UML 2.5.1 §§7.3.4, 9.2.4, 9.6.4, 9.8.4, 10.4.4 and 11.5.4. LAML draws these class and instance symbols with measured text and checked relationships:

| Drawing | Source or style |
|---|---|
| Classifier and typed member compartments | `class Kunde {` followed by `name: String` on the next line |
| Interface, enumeration and data type | `interface API`, `enum Status`, `datatype Geld` |
| Abstract classifier | `abstract class Dienst` |
| Dashed template-parameter corner box | `class Repository<T>` |
| Underlined object header and literal slots | `kunde1(of=Kunde) {` followed by `name = "Ada"` on the next line |
| Binary association and end labels | `@orders Kunde "1" -- "0..*" Bestellung` |
| Composition and aggregation | `A *-- B` / `A o-- B` |
| Generalization and realization | `A --|> B` / `A ..|> B` |
| Qualifier box | `qualifier(id:UUID)` on a class-association end |
| N-ary association | `association Verordnet` with three or more arms |
| Association class | `Facts .. relation[orders]` |
| Compact interface realization | Named relation styled with `interface-notation: lollipop` |
| Template binding | `StringRepository ..> Repository <<bind(T->String)>>` |

See the [class and object guide](class-object-diagrams.md), [class example](../examples/class-basic.laml) and [object example](../examples/object-basic.laml).
