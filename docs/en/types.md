---
title: Types
parent: Language Guide
grand_parent: English Docs
nav_order: 3
---

# Types

PrSM is statically typed. Every field, parameter, and return position has a type that is checked at compile time.

## Primitive types

| PrSM | C# equivalent |
|---|---|
| `Int` | `int` |
| `Float` | `float` |
| `Double` | `double` |
| `Bool` | `bool` |
| `String` | `string` |
| `Long` | `long` |
| `Byte` | `byte` |
| `Unit` | `void` |

## Unity and .NET types

Unity types are used directly by name. PrSM does not alias or wrap them.

```prsm
serialize target: Transform = null
serialize body: Rigidbody = null
val offset: Vector3 = Vector3.zero
```

Supported families include `MonoBehaviour`, `ScriptableObject`, `Transform`, `Rigidbody`, `Rigidbody2D`, `Animator`, `AudioSource`, `Camera`, `Canvas`, `Vector2/3/4`, `Quaternion`, `Color`, `Rect`, and most other common Unity types.

## Nullability

Type annotations without `?` are non-nullable by default. The compiler tracks this through field qualifiers and performs null-safety checks.

```prsm
require rb: Rigidbody      // non-null, resolved in Awake
optional cam: Camera?      // nullable, may be absent
```

- `Type` — non-nullable
- `Type?` — nullable; must be guarded before use
- `?.` safe-call and `?:` null-coalescing are provided for nullable access
- `!!` non-null assertion is available as an escape hatch

## Generic types

PrSM supports generic type references and lowers them to their .NET equivalents:

| PrSM | C# |
|---|---|
| `Array<T>` | `T[]` |
| `List<T>` | `List<T>` |
| `Map<K, V>` | `Dictionary<K, V>` |
| `Set<T>` | `HashSet<T>` |
| `Queue<T>` | `Queue<T>` |
| `Stack<T>` | `Stack<T>` |
| `Seq<T>` | `IEnumerable<T>` |

```prsm
var items: List<Int> = null
var lookup: Map<String, Float> = null
```

## Type inference

Local variable types can often be omitted when the right-hand side makes the type unambiguous, but explicit annotations are also always valid.
