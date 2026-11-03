# UML 2.5.1 profile notation

Reference: OMG UML 2.5.1 §12.3.4 and Annex A. LAML draws these profile-definition symbols:

| Drawing | Source |
|---|---|
| Tabbed `«profile»` package | `profile Audit { ... }` |
| Contained `«stereotype»` classifier | `stereotype Tracked` inside `Audit` |
| Declared `«metaclass»` classifier | `metaclass Class` |
| Filled-triangle extension | `Audit.Tracked --^> Class` |
| Dashed metaclass reference | `Audit ..> Class <<reference>>` |
| Dashed profile application | `Commerce ..> Audit <<apply>>` or `<<strict>>` |

The compiler checks endpoint kinds and mandatory keywords. See the [profile guide](profile-diagrams.md) and [audit example](../examples/profile-audit.laml).
