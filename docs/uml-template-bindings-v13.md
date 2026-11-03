# UML 2.5.1 template binding notation

Reference: OMG UML 2.5.1 §7.3.4. A dashed dependency marked `«bind»` points from the bound class to its template and names its substitutions:

```laml
settings {
    type: class
}
semantics {
    class Repository<T>
    class StringRepository
    @binding StringRepository ..> Repository <<bind(T->String)>>
}
```

LAML checks the locally declared classes, template parameter name, substitution uniqueness and dependency arrow. In class diagrams, `Repository<T>` shows its formal parameter in a dashed corner box. See the [binding example](../examples/class-binding.laml).
