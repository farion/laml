# UML 2.5.1 deployment-notation audit

This is a **notation** audit, not a claim of UML abstract-syntax or runtime conformance. Reference: OMG *Unified Modeling Language* 2.5.1 §§19.2.4 (targets, deployment arrows and nesting), 19.3.4 (artifacts and manifestations), 19.4.4 (nodes, devices, execution environments and communication paths).

| UML image notation | LAML source and output | Boundary |
|---|---|---|
| Deployed target | `node Name` draws a measured perspective cube. | No instance-level `:name` or underlining semantics. |
| Device and execution environment | `device Name` and `executionEnvironment Name` draw cubes with `«device»` and `«executionEnvironment»`. | Extra reserved keywords cannot masquerade as another kind. |
| Nested Node | Node/deployment target declarations inside a Node cube. | No general UML owned-property or structural-feature metamodel. |
| Artifact | `artifact Name` draws a classifier rectangle with `«artifact»`. | Optional document icon and inferred physical file properties omitted. |
| Contained deployment | Artifact declared inside a Node cube is drawn inside it as an implicit deployment. | No redundant internal `«deploy»` line or runtime installation. |
| External deployment | `Artifact ..> Node <<deploy>>` draws a dashed open-arrow dependency labeled `«deploy»`, artifact-to-target. | Opposite direction, missing keyword and a component as deployment target fail. |
| Manifestation | `Artifact ..> Component <<manifest>>` draws a dashed open-arrow dependency labeled `«manifest»`. | This does not mean the component is running on that node. |
| Communication path | `Node -- Node` draws a solid association stroke with an optional name. | Roles, multiplicities, qualifiers, and advanced association semantics are outside the subset. |
| Notes and packages | Dotted note attachments and scoped package containers reuse graph geometry. | A package is not a physical deployment target; a component nested directly inside a node is rejected. |

Renderer geometry treats the cube's top/right faces as part of the node's collision box. Routes parallel within 4 px of a rectangular deployment-element outline are rejected rather than merging with its stroke. Shared graph routing remains deterministic and can report that an otherwise valid relationship needs more space.
