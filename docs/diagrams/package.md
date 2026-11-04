# Package diagrams

## Basic example

```laml
settings {
    type: package
}
semantics {
    package Shop {
        package Orders
    }
    package Payments
    Shop.Orders ..> Payments
}
layout {
    Shop.Orders < Payments : align=center; gap=130
}
```

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../images/package-quickstart-modern-dark.png">
  <img src="../images/package-quickstart-modern.png" alt="Nested Shop package with a dependency on Payments">
</picture>

[Complete source](../../examples/package-quickstart.laml). Braces enclose a package; `Shop.Orders` addresses its nested package. `..>` is a dependency. The placement rule aligns the two package centers so the dependency runs horizontally.

## Imports, access and merge

```laml
// Excerpts from examples/package-architecture.laml
semantics {
    // […]
    Shop.API ..> Payments <<import>>
    Shop.Orders ..> Common <<access>>
    Shop.Pricing ..> Common <<merge>>
}
```

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../images/package-architecture-modern-dark.png?v=ea7813c">
  <img src="../images/package-architecture-modern.png?v=ea7813c" alt="Nested packages with import, access and merge">
</picture>

[Complete source](../../examples/package-architecture.laml).

| Form | Drawing |
|---|---|
| `package Name` | Leaf package. |
| `package Name { ... }` | Container package (body on separate lines). |
| `A ..> B` | Dependency. |
| `A ..> B <<import>>` | Public package import. |
| `A ..> B <<access>>` | Private package access. |
| `A ..> B <<merge>>` | Package merge. |

Placement, routes and LSS selectors: [GNE](gne.md). [Full syntax](../language-reference.md#package-diagrams).
