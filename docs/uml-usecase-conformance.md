# UML 2.5.1 use-case notation audit

LAML produces checked use-case **images**, not a UML model. Reference: OMG UML 2.5.1 use-case notation in chapter 18 and generalization/dependency notation.

| Visible notation | Source | Enforced boundary |
|---|---|---|
| Actor | `actor Customer` (stick figure, caption beneath). | Actor remains outside the system boundary; no actor classifier compartments. |
| Subject boundary | `system Shop { ... }` (named rectangle enclosing use cases). | One subject per diagram in this subset, no nested subjects. Visual containment and source scope do not assert UML ownership. |
| Use case | `usecase Order(label="Place order")` (ellipse). | Must be inside the subject; no extension points, ownership variants or full classifier semantics. |
| Association | `Customer -- Shop.Order` (plain solid line). | Only between actor and use case. |
| Include | `Order ..> Payment <<include>>` (dashed arrow, `«include»` label). | Arrow points to included use case; both within the same subject. |
| Extend | `Discount ..> Order <<extend>>` (dashed arrow, `«extend»` label). | Arrow points to extended base use case; no guards or extension-point compartment. |
| Generalization | `Special --|> General` (solid open-triangle arrow). | Same-kind actor or same-subject use cases only; cycles rejected. |
| Note | `Note .. UseCase` (unadorned dotted annotation). | Never treated as association or dependency. |

Arbitrary `..>` dependencies, directed actor associations and dashed stand-ins for generalization are not silently converted into UML meanings. This subset does not claim behavioral or requirements completeness.
