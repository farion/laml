# UML 2.5.1 composite-structure notation audit

Reference: OMG UML 2.5.1 §§11.2.3–11.2.4 (structured classifiers, internal structure, parts, roles and connectors), 11.3.4 (ports), and Figure 11.2–11.4. LAML produces checked visual notation, not a complete UML model.

| UML notation | LAML source and output | Checked boundary |
|---|---|---|
| Structured classifier | `structure Checkout { ... }` → a classifier rectangle with a named **internal structure** compartment | Exactly one root owner with contained roles; no association or dependency is inferred from containment. |
| Composite part | `part store(of=Store)` → `store:Store` in a **solid** part box | Type is a locally declared class; composition is distinguished from an ordinary reference role. |
| Noncomposite role | `role gateway(of=Gateway)` → `gateway:Gateway` in a **dashed** role box | A dashed role outline is not a dashed graph dependency. |
| Port | `port In(provides=API)` or `port Out(requires=API)` → square on classifier/role boundary with a visible label | One declared interface per port; copied from its locally declared class type to each role. |
| Assembly | `gateway::Backend == store::In` → plain connector between compatible simple ports | Sibling roles, opposite provided/required roles, same interface. Not an association between classifier types. |
| Delegation | `Checkout::Front => gateway::Public` → plain connector from owner's boundary port to its own role | Same provided/required role and interface; no runtime forwarding is simulated. |

UML also permits connector-end and property multiplicities, connector typing by associations, n-ary connectors, complex ports and additional compact ball/socket glyphs. This first subset rejects these rather than silently substituting a different graphic. Any required hidden element or unsafe connector crossing through the compulsory compartment label fails at build time.
