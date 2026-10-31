# v1.4 component diagrams — implementation boundary

LAML remains an image language, not a repository of UML model elements. This is the next diagram type after class, object and sequence. The first target is a useful component diagram, not a generic graph with a `«component»` string. OMG UML 2.5.1 §11.6.4 is the notation reference.

## Authoring target

```text
settings {
    type: component
}
semantics {
    component Gateway {
        @http port HTTP(requires=OrderAPI)
    }
    component Orders {
        @incoming port Incoming(provides=OrderAPI)
    }
    interface OrderAPI
    @provides Orders ..|> OrderAPI
    @needs Gateway ..> OrderAPI
    @calls Gateway ..> Orders : place order
}
layout {
    Gateway < Orders : gap=180
}
styling {
    relation[provides] {
        color: "#395373"
    }
}
```

The example builds as SVG, PNG and PDF. The three published examples cover the overview, compact interfaces and contained parts separately.

## Visible notation and rules

1. `component` is a classifier rectangle with a measured `«component»` keyword. The optional component icon, when added, is a UML glyph, not a Phosphor decoration. Body attributes/operations follow the safe class subset; ports have addressable IDs and remain on the actual boundary.
2. Component-to-component `..>` is a dependency with its existing dashed open arrow. Component-to-interface `..|>` denotes a provided interface in box form; `..>` denotes a required interface in box form. Compact provided/required symbols are local to the component or its simple port; they must not make a shared interface box disappear merely because one relation opts into compact notation.
3. Assembly connects compatible simple ports on **parts within a containing component's internal structure**. Delegation joins the containing component's boundary port to an internal part's port. These are distinct from dependencies between top-level component definitions; neither is inferred from an arbitrary `--`. Ball-and-socket is an optional drawing of a supported simple-port assembly, not a shortcut for an untyped line.
4. Packages, notes, scoped IDs, layout, styling and SVG/PNG/vector-PDF reuse the graph core. Semantics stay in the source; mandatory placement cannot be overridden by a relation operator.
5. Reject untyped ball/socket pairs, conflicting ownership, mixed box/compact uses of a hidden interface, unsupported port typing, and any assembly/delegation arrangement whose notation cannot be routed without a false junction. Do not infer provided/required interfaces solely from a generic dependency.

## Concrete build and acceptance plan

1. **Overview and interfaces (implemented and build-verified):** parse `type: component`; accept components, interfaces, packages and notes; draw measured `«component»` headings; distinguish dependencies from provided/required box interfaces; support compact lollipop and socket only on checked, otherwise-unused interface boxes. Example and positive/negative tests must build to SVG/PNG/PDF.
2. **Ports (implemented and build-verified):** require `port Name(provides=Interface)` or `port Name(requires=Interface)` on components; resolve the declared interface within package scope; measure the visible contract label. Reuse generic port side mandates, without silently assigning UML meaning to generic ports. Reject missing, conflicting or unknown contracts.
3. **Internal wiring (implemented and build-verified):** introduce explicit parts owned by a component, separate dependency from assembly/delegation operators, and validate that simple assembly ends expose opposite roles for the same interface ID. Delegation must connect an outer boundary port to an internal part's port in the correct ownership context. Keep optional ball/socket assembly notation distinct from a generic line. Add collision-safe routes and failing cases before documenting this as supported.
4. **Finish (implemented and locally verified):** the optional UML corner glyph clears headings and yields to top ports; three representative source diagrams, PNG previews, SVG/PNG/PDF exports, conformance notes and documentation are in the repository. All Rust source files stay within 500 lines. Keep publication separate from local implementation; do not push.

**90% target met as an authoring-use-case judgment:** the three representative architectural drawings above build with checked notation. This is not a percentage claim about the UML standard.

**Out of scope:** a complete UML Component metamodel, automatic realization discovery, operation compatibility checking, runtime wiring semantics, and optional white-box compartments whose content cannot be validated from the diagram source.
