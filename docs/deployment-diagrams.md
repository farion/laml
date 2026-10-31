# UML deployment diagrams

Set `type: deployment` in `settings`. LAML generates an image of a checked UML 2.5.1 deployment-notation subset, not a software installation or an executable UML model. These diagrams share GNE container placement, safe routing, scoped IDs, LSS styling and SVG/PNG/vector-PDF exports.

## Infrastructure and contained artifacts

`node Cluster` is an unqualified UML Node; `device Host` and `executionEnvironment JVM` display their UML keywords. All three use a measured perspective cube. Put a node or artifact inside a device or execution environment to show its nested infrastructure or implicit artifact deployment. The outer node contains only other nodes and deployed artifacts; an ordinary `component` cannot be silently nested as if it were a deployed artifact. Artifact declarations display `«artifact»` in a classifier rectangle. `package Name { ... }` provides source-scoped grouping, not a substitute for a deployment target. See [nested topology](../examples/deployment-topology.laml).

## Distinct deployment relationships

```text
settings {
    type: deployment
}
semantics {
    device Server {
        executionEnvironment JVM {
            artifact WorkerJar
        }
    }
    device Database
    artifact GatewayJar
    component Gateway
    @link Server -- Database : private link
    @runs GatewayJar ..> Server <<deploy>>
    @builds GatewayJar ..> Gateway <<manifest>>
}
```

- `Node -- Node` is a **solid communication path**, optionally named or labeled. It is not a generic dependency.
- `Artifact ..> Node <<deploy>>` is a dashed, open-arrow **deployment** from the artifact to its target. Alternatively, placing an artifact inside a node depicts the deployment without adding a second arrow.
- `Artifact ..> Component <<manifest>>` is a dashed, open-arrow **manifestation**. The component is the model element represented by the artifact, **not** the deployment target. The [manifestation example](../examples/deployment-manifest.laml) shows both relationships from one artifact.
- `Note .. Node` is a dotted, unadorned note attachment, distinct from all three relations above.

Fully qualified paths such as `Server.JVM.WorkerJar` address contained elements in layout and relations. `@id` remains diagram-wide, and the nearest container's local names take precedence otherwise. Explicit mandatory placement uses the same `layout` syntax as other graph diagrams. A fixed-size cube that cannot enclose its descendants fails rather than clipping them. See [the overview example](../examples/deployment-basic.laml).

## Subset boundaries

These are **type-level images**: LAML does not instantiate a runtime cluster, distinguish node/artifact instances with an instance model, calculate configuration from a `DeploymentSpecification`, or infer artifact/component relationships from a file system. Communication paths in this subset have a plain association stroke with optional name/label but no checked association-end adornments. Artifact file names can be visible labels (`artifact App(label="app.jar")`); they are not read or resolved from disk. Extra stereotypes cannot replace or contradict the UML node, artifact, `«deploy»`, or `«manifest»` keywords. A node box uses a measured perspective glyph; LSS cannot swap it for an ellipse, diamond, or note shape. Unsupported notation and unsafe routing report errors. The [notation audit](uml-deployment-conformance.md) details the exact boundary against UML 2.5.1 §§19.2–19.4.
