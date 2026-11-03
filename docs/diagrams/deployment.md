# Deployment diagrams

## Basic example

```laml
settings {
    type: deployment
}
semantics {
    device Server {
        artifact App
    }
    device Database
    Server -- Database : TLS
}
layout {
    Server < Database : gap=130
}
```

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../images/deployment-quickstart-modern-dark.png">
  <img src="../images/deployment-quickstart-modern.png" alt="Two devices, a contained artifact and a communication path">
</picture>

[Complete source](../../examples/deployment-quickstart.laml). The artifact is drawn inside its device; `--` connects devices with a communication path.

## Nested topology, deployment and manifestation

```laml
// Excerpts from examples/deployment-basic.laml
semantics {
    device Edge {
        executionEnvironment Runtime {
            artifact WorkerJar
        }
    }
    // […]
    @runs GatewayJar ..> Edge <<deploy>>
    @realizes GatewayJar ..> Gateway <<manifest>>
}
```

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../images/deployment-basic-modern-dark.png">
  <img src="../images/deployment-basic-modern.png" alt="Deployment and manifestation beside contained runtime">
</picture>

[Complete source](../../examples/deployment-basic.laml). `<<deploy>>` points to a deployment target; `<<manifest>>` points to the component realized by an artifact.

```laml
// Excerpt from examples/deployment-topology.laml
semantics {
    node Region {
        device Edge {
            executionEnvironment JVM {
                artifact GatewayJar
            }
        }
        device Database
        device Monitor
        @database Edge -- Database : private link
        @metrics Edge -- Monitor : metrics
    }
}
```

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../images/deployment-topology-modern-dark.png">
  <img src="../images/deployment-topology-modern.png" alt="Nested deployment topology">
</picture>

[Complete source](../../examples/deployment-topology.laml).

## Deployment versus manifestation

```laml
// Excerpt from examples/deployment-manifest.laml
component Billing
artifact BillingJar(label="billing.jar")
executionEnvironment JVM
@manifestation BillingJar ..> Billing <<manifest>>
@deployment BillingJar ..> JVM <<deploy>>
```

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../images/deployment-manifest-modern-dark.png">
  <img src="../images/deployment-manifest-modern.png" alt="Artifact manifested from a component and deployed on a JVM">
</picture>

[Complete source](../../examples/deployment-manifest.laml). Manifestation points to a component; deployment points to a node or execution environment. Use [GNE](gne.md) for shared placement and styling. [Full deployment syntax](../language-reference.md#deployment-diagrams).
