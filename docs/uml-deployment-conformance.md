# UML 2.5.1 deployment notation

Reference: OMG UML 2.5.1 §§19.2.4, 19.3.4 and 19.4.4. LAML draws these deployment symbols and validates their endpoint types:

| Drawing | Source |
|---|---|
| Perspective node cube | `node Name` |
| Device and execution environment | `device Host`, `executionEnvironment JVM` |
| Contained node or artifact | Declare it inside the enclosing node's block |
| Artifact classifier | `artifact App` |
| Deployment arrow | `Artifact ..> Node <<deploy>>` |
| Manifestation arrow | `Artifact ..> Component <<manifest>>` |
| Communication path | `Node -- Node` |
| Dotted note attachment | `Hint .. Node` |

See the [deployment guide](deployment-diagrams.md) and [manifestation example](../examples/deployment-manifest.laml).
