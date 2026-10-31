# v1.3 template binding notation audit

Primary source: OMG *Unified Modeling Language 2.5.1*, §7.3.4, available at https://www.omg.org/spec/UML/2.5.1/PDF.

The specification shows a template binding as a dashed arrow from the bound element to its template, marked `«bind»`, with substitutions written `formal -> actual`. LAML's checked subset uses:

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

LAML checks that the arrow is a dashed dependency, the source and target are locally declared classes, the target has the named formal parameter, and each formal occurs once. It does not resolve actual argument types or synthesize an expanded bound classifier. A bare `<<bind>>` is rejected rather than presented as a validated binding.

**Notation limit:** UML 2.5.1 also depicts formal template parameters in a dashed rectangle on a classifier's upper-right corner. LAML v1.3 currently displays `Repository<T>` inline as agreed in the language design. Do not claim complete UML template-notation conformance for this subset.
