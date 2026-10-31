# UML package diagrams

Set `type: package` for packages and checked package-to-package relationships. LAML produces an image, not a UML namespace resolver or package-merge engine. The complete [architecture example](../examples/package-architecture.laml) builds as SVG, PNG and vector PDF.

```text
settings {
    type: package
}
semantics {
    package Shop {
        package API
        package Orders
        package Pricing
        Orders ..> Pricing
    }
    package Payments
    package Common
    Shop.API ..> Payments <<import>>
    Shop.Orders ..> Common <<access>>
    Shop.Pricing ..> Common <<merge>>
}
```

`package Name` draws a leaf folder symbol with its name in the main rectangle. `package Name { ... }` contains nested packages (and optionally notes) and puts its name in the tab; names are scoped (`Shop.API`), while an explicit `@id` is diagram-wide. UML package visibility can be shown on nested packages as `+package PublicPart` or `-package PrivatePart`; no classifier contents or arbitrary node kinds are silently accepted. A fixed package frame too small to fit its visible contents fails. `note Hint "text"` can be linked to a package through `Hint .. Shop.API` without becoming a dependency.

Every package relationship uses a **dashed directed open arrow**; the source and target give its meaning:

- `A ..> B`: ordinary, **unmarked dependency**, *not* a package import.
- `A ..> B <<import>>`: public package import, arrow from importer **A** to imported **B**.
- `A ..> B <<access>>`: private package import, same direction but distinct keyword.
- `A ..> B <<merge>>`: package merge, arrow from receiving **A** to merged **B**.

Self-relationships and importing/accessing a package's own nested package are rejected; merge cycles also fail. Other stereotypes, class relationships, bare solid arrows and inappropriate endpoints are rejected. Source paths and displayed arrow labels are checked as notation, but imports do not make names available to the compiler and merges do not combine definitions. See the [notation audit](uml-package-conformance.md).
