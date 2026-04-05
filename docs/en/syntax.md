---
title: Syntax
parent: Language Guide
grand_parent: English Docs
nav_order: 1
---

# Syntax

PrSM keeps its surface syntax small and regular. There are no semicolons, no parentheses on control flow, and no mandatory boilerplate around the common Unity patterns.

## File structure

Each `.prsm` file contains exactly one top-level declaration. Files typically start with `using` imports followed by the single declaration body.

```prsm
using UnityEngine
using System.Collections

component PlayerController : MonoBehaviour {
    serialize speed: Float = 5.0

    update {
        move()
    }

    func move() {
        transform.Translate(Vector3.forward * speed * Time.deltaTime)
    }
}
```

## Key rules

- **Newline-terminated statements** — no semicolons
- **Brace-based blocks** — `{}` delimit all bodies
- **Parenthesis-free control flow** — `if`, `when`, `for`, `while` do not wrap conditions in `()`
- **One declaration per file** — the top-level type is the file unit
- **`using` for imports** — brings Unity and .NET namespaces into scope

## Identifiers and naming

- Type names are `PascalCase`
- Member names and field names are `camelCase`
- Lifecycle keywords (`awake`, `update`, etc.) are lowercase reserved words

## Comments

```prsm
// single-line comment

/*
  multi-line comment
*/
```

## Minimal single-file example

```prsm
using UnityEngine

component Empty : MonoBehaviour {
    update {
    }
}
```

The generated C# for this is a normal `MonoBehaviour` subclass with an `Update()` method.
