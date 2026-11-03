# UML profile diagrams

Set `type: profile` to draw a UML 2.5.1 profile-definition diagram. A profile is a tabbed package with a visible `«profile»` keyword. Its directly contained stereotypes are classifiers labeled `«stereotype»`; external declared UML metaclasses appear as classifiers labeled `«metaclass»`. A plain package may show where the profile is applied.

```text
settings {
    type: profile
}
semantics {
    profile Audit {
        stereotype Tracked
    }
    metaclass Class
    package Commerce
    Audit.Tracked --^> Class
    Audit ..> Class <<reference>>
    Commerce ..> Audit <<apply>>
}
layout {
    Audit < Class : gap=260
    Commerce below Audit : gap=160
}
```

[Build this example](../examples/profile-audit.laml) as SVG, PNG or vector PDF. The profile contains `Audit.Tracked`, not the UML metaclass `Class`. The solid `--^>` arrow has a **filled triangle pointing to the extended metaclass**; `--|>` (hollow generalization) is not a substitute. A separately declared `Audit ..> Class <<reference>>` draws the optional, dashed open-arrow metaclass reference and does not declare an extension. `Commerce ..> Audit <<apply>>` draws a dashed open-arrow profile application from the package to the profile; `<<strict>>` is the alternative label. The source operators express notation, not a geometric placement constraint.

The checked subset accepts one or more profiles, each with a directly owned leaf stereotype; each stereotype needs an explicit extension. Each metaclass is an external leaf from this known UML set: `Activity`, `Artifact`, `Association`, `Class`, `Classifier`, `Component`, `Dependency`, `Element`, `Interface`, `NamedElement`, `Operation`, `Package`, `Port`, `Property`, `State`, `UseCase`. Packages receiving a profile are external leaf packages. Profile/metaclass references and package/profile applications are optional, explicit relations. Duplicate or mismatched extensions, references or applications fail. Text keywords, outlines and relationships cannot be hidden with styles, and unsafe geometry fails.

See the [notation audit](uml-profile-conformance.md) for the checked drawing rules.
