---
title: Strings & Collections
parent: Language Guide
grand_parent: English Docs
nav_order: 4
---

# Strings & Collections

## String literals

Strings use double quotes:

```prsm
val greeting = "Hello, world!"
```

## String interpolation

PrSM supports two interpolation forms inside string literals:

| Syntax | Use |
|---|---|
| `$name` | inserts a simple identifier |
| `${expression}` | inserts any expression |

```prsm
func statusLine(): String = "HP: $hp / $maxHp"
func debugInfo(): String = "pos=${transform.position} vel=${rb.velocity.magnitude:.2}"
```

Interpolated strings lower to C# string interpolation (`$"..."`).

## Collections

PrSM recognizes a useful set of generic collection types and lowers them to their .NET equivalents:

| PrSM | Generated C# |
|---|---|
| `Array<T>` | `T[]` |
| `List<T>` | `System.Collections.Generic.List<T>` |
| `Map<K, V>` | `System.Collections.Generic.Dictionary<K, V>` |
| `Set<T>` | `System.Collections.Generic.HashSet<T>` |
| `Queue<T>` | `System.Collections.Generic.Queue<T>` |
| `Stack<T>` | `System.Collections.Generic.Stack<T>` |
| `Seq<T>` | `System.Collections.Generic.IEnumerable<T>` |

```prsm
var enemies: List<GameObject> = null
var scoreByPlayer: Map<String, Int> = null
var visitedRooms: Set<String> = null
```

Collection fields work with Unity serialization the same way their C# counterparts do. `List<T>` fields marked `serialize` appear in the Inspector.

## Working with collections

Because collections lower to standard .NET types, all .NET instance methods are available through `intrinsic` or direct method calls when needed:

```prsm
enemies.Add(newEnemy)
enemies.Remove(deadEnemy)
val count = enemies.Count
```
