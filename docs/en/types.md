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

Local variable types can often be omitted when the right-hand side makes the type unambiguous:

```prsm
val name = "Player"       // inferred as String
val hp = 100              // inferred as Int
val speed = 5.0           // inferred as Float
var alive = true          // inferred as Bool
```

Explicit annotations are always valid and sometimes required (e.g., when the initializer is `null`).

### Generic type inference (PrSM 2 부터)

v2 introduces limited context-based inference for generic method calls. See [Generic Inference](generic-inference.md) for details.

```prsm
val rb: Rigidbody = get()        // inferred: GetComponent<Rigidbody>()
val health: Health? = child()    // inferred: GetComponentInChildren<Health>()
```

## Null safety model

PrSM enforces null safety at compile time through type annotations.

**Non-nullable types** (`Type`) guarantee a value is present:

```prsm
require rb: Rigidbody     // guaranteed non-null after Awake
val speed: Float = 5.0    // never null
```

**Nullable types** (`Type?`) must be guarded before use:

```prsm
optional cam: Camera?     // may be null

// Safe access patterns:
cam?.enabled = false          // safe call — no-op if null
val depth = cam?.depth ?: 0   // elvis — fallback value
val fov = cam!!.fieldOfView   // assert non-null (warning W001 if already non-null)
```

**Safe call chain**: `a?.b?.c?.d` short-circuits on the first null.

**Non-null assertion** (`!!`): Bypasses null checking. Emits warning W001 if used on a type that is already non-null.

## Type conversions

PrSM does not have implicit conversions. All type relationships pass through to C#. For instance, `Int` to `Float` requires an explicit cast via `intrinsic` or letting C# handle it when the generated code assigns to a typed position.

## Complete type mapping reference

| PrSM | C# | Category |
|---|---|---|
| `Int` | `int` | Primitive |
| `Float` | `float` | Primitive |
| `Double` | `double` | Primitive |
| `Bool` | `bool` | Primitive |
| `String` | `string` | Primitive |
| `Char` | `char` | Primitive |
| `Long` | `long` | Primitive |
| `Byte` | `byte` | Primitive |
| `Unit` | `void` | Return type |
| `Array<T>` | `T[]` | Collection |
| `List<T>` | `System.Collections.Generic.List<T>` | Collection |
| `Map<K,V>` | `System.Collections.Generic.Dictionary<K,V>` | Collection |
| `Set<T>` | `System.Collections.Generic.HashSet<T>` | Collection |
| `Queue<T>` | `System.Collections.Generic.Queue<T>` | Collection |
| `Stack<T>` | `System.Collections.Generic.Stack<T>` | Collection |
| `Seq<T>` | `System.Collections.Generic.IEnumerable<T>` | Collection |
| *Other* | *Passes through unchanged* | Unity/.NET |
