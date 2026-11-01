# UML 2.5.1 profile diagram notation audit

Sources: OMG UML 2.5.1 §12.3.4 (Notation), §12.3.3.3 (ProfileApplication), §12.3.3.6 (Extensions) and Annex A (Profile Diagram under Packages).

| UML element | LAML notation | Checked scope |
|---|---|---|
| Profile | `profile Audit { ... }` | Tabbed package with visible `«profile»` heading; direct stereotype ownership. |
| Stereotype | `stereotype Tracked` inside `Audit` | Classifier with visible `«stereotype»` heading. No uninterpreted stereotype attributes. |
| Referenced metaclass | `metaclass Class` outside profile | Classifier with visible `«metaclass»` heading; only the explicitly listed UML metaclasses are recognized. |
| Extension | `Audit.Tracked --^> Class` | Solid line from stereotype to metaclass, with a **filled** triangular arrowhead. No inferred required extension or generalization. |
| Metaclass reference | `Audit ..> Class <<reference>>` | Optional dashed open-arrow reference from profile to metaclass; not interchangeable with extension. |
| Profile application | `Commerce ..> Audit <<apply>>`, or `<<strict>>` | Dashed open-arrow from package to profile, checked source, target and keyword. Strict filtering is not executed. |

No complete UML metamodel, XMI interchange or runtime application is implied. We reject unsupported optional annotations and notation rather than showing an unmarked dependency, hollow generalization, arbitrary stereotype label or generic package as an equivalent graphic. Measured geometry and retained mandatory marks are checked during build.
