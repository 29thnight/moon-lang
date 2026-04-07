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

## Raw string literals (since PrSM 4)

Triple-quoted strings preserve newlines and special characters without escaping. Leading whitespace is trimmed to the closing delimiter's indentation level. Interpolation (`$var`, `${expr}`) is still active inside raw strings.

```prsm
val json = """
    {
        "name": "Player",
        "level": 42
    }
    """

val query = """
    SELECT * FROM users
    WHERE name = '${userName}'
    """
```

Lowers to a C# 11 raw string literal where supported, falling back to `@"..."` verbatim strings on older targets. Unterminated raw strings produce E128.

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

## Collection literals (since PrSM 4)

PrSM 4 adds inline syntax for `List<T>`, `Map<K, V>`, `Array<T>`, and `Set<T>`:

```prsm
val numbers = [1, 2, 3, 4, 5]
val names = ["Alice", "Bob", "Charlie"]
val empty: List<Int> = []

val lookup = {"hp": 100, "mp": 50}
val arr: Array<Int> = arrayOf(1, 2, 3)
val unique = setOf("Alice", "Bob", "Alice")
```

Generated C#:

```csharp
var numbers = new List<int> { 1, 2, 3, 4, 5 };
var lookup = new Dictionary<string, int> { ["hp"] = 100, ["mp"] = 50 };
var arr = new int[] { 1, 2, 3 };
var unique = new HashSet<string> { "Alice", "Bob", "Alice" };
```

Element type is inferred from the first element. Empty literals require an explicit type annotation (E107). Mixed element types produce E108.

## `in` membership operator (since PrSM 4)

`in` tests membership against ranges, lists, and maps:

```prsm
if x in 1..10 { log("In range") }
if name in ["Alice", "Bob"] { log("Known user") }
if key in lookup { log("Key exists") }
```

Generated C#:

```csharp
if (x >= 1 && x <= 10) { Debug.Log("In range"); }
if (new List<string>{"Alice", "Bob"}.Contains(name)) { Debug.Log("Known user"); }
if (lookup.ContainsKey(key)) { Debug.Log("Key exists"); }
```

`in` on a type that lacks both `Contains` and `ContainsKey` produces E129.

## Working with collections

Because collections lower to standard .NET types, all .NET instance methods are available through `intrinsic` or direct method calls when needed:

```prsm
enemies.Add(newEnemy)
enemies.Remove(deadEnemy)
val count = enemies.Count
```
