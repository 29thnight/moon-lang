---
title: Strings & Collections
parent: Language Guide
grand_parent: English Docs
nav_order: 4
---

# Strings & Collections

## Strings

PrSM supports plain string literals and interpolation.

```prsm
serialize name: String = "test"
func label(): String = "HP: $hp / ${maxHp}"
```

Interpolation forms:

- `$name`
- `${expression}`

## Collections and array-like types

PrSM does not currently ship a separate standard library manual, but it already understands a useful set of collection-oriented type names.

- `Array<T>` -> `T[]`
- `List<T>` -> `System.Collections.Generic.List<T>`
- `Map<K, V>` -> `System.Collections.Generic.Dictionary<K, V>`
- `Set<T>` -> `System.Collections.Generic.HashSet<T>`
- `Queue<T>` and `Stack<T>` are recognized
- `Seq<T>` -> `System.Collections.Generic.IEnumerable<T>`

Example:

```prsm
var items: List<Int> = null
var lookup: Map<String, Int> = null
var tags: Set<String> = null
```
