# UML 2.5.1 package notation audit

LAML draws checked **package images**, not UML package import, visibility, or merge semantics. References: OMG UML 2.5.1 §§7.4.4.3 (package imports) and 12.2.4 (package and merge notation).

| Visible notation | LAML source | Boundary |
|---|---|---|
| Empty package | `package A` (folder tab, name in main rectangle). | Package contents are not inferred. |
| Nested package | `package A { package B }` on separate lines (name in enclosing tab, child within rectangle). | Scoped source IDs indicate containment; only packages and notes are supported. |
| Unmarked dependency | `A ..> B` (dashed open arrow, no keyword). | Does **not** imply import, access or merge. |
| Public package import | `A ..> B <<import>>` (dashed arrow and `«import»`). | Direction from importing to imported package; no name resolution, re-export computation or aliases. |
| Private package access | `A ..> B <<access>>` (dashed arrow and `«access»`). | Same direction, distinct UML visibility; not executed. |
| Package merge | `A ..> B <<merge>>` (dashed arrow and `«merge»`). | Direction from receiving to merged package; no member combination. |
| Note | `Hint .. A` (unmarked dotted line). | Annotation only, not a dependency. |

Classifiers in packages, element imports, aliases, textual imports, a model corner triangle, freeform keywords and executable namespace semantics are outside this first subset. Self-links, importing one's own contained package and merge cycles are rejected rather than assigning them misleading meaning.
