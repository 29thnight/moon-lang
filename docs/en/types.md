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

### Generic type inference (since PrSM 2)

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

### Casting operators (since PrSM 4)

PrSM 4 introduces explicit cast operators and conversion methods:

| Form | Behavior |
|---|---|
| `expr as Type?` | Safe cast — returns `null` if the cast fails |
| `expr as! Type` | Force cast — throws `InvalidCastException` if the cast fails |
| `expr.toInt()` `.toFloat()` `.toDouble()` `.toString()` | Explicit numeric/string conversion |

```prsm
val enemy = collider as Enemy?      // Enemy or null
val boss = collider as! Boss        // throws on mismatch
val pixels = 42.toFloat()           // 42.0f
```

### Smart casts (since PrSM 4)

After an `is` check, the variable is narrowed to the checked type within the same scope:

```prsm
if collider is BoxCollider {
    log(collider.size)   // collider has type BoxCollider here
}

when target {
    is Enemy => target.takeDamage(10)
    is Ally  => target.heal(5)
}
```

E109 is raised when `as!` is used to cast to a provably unrelated type. W021 warns when an `as?` result is never null-checked.

## Tuples (since PrSM 4)

Tuples group multiple values into a single compound type. Both positional and named tuples are supported, and tuples can be destructured into separate variables.

```prsm
func getResult(): (Int, String) = (42, "answer")
val (num, name) = getResult()

func getStats(): (hp: Int, mp: Int) = (hp: 100, mp: 50)
val stats = getStats()
log(stats.hp)
```

Tuples lower to C# `ValueTuple`. Destructuring count must match (E117); named-tuple field access with the wrong label produces E118.

## Type aliases (since PrSM 4)

`typealias` introduces a compile-time alias for an existing type. Aliases are erased during lowering — they have no runtime cost.

```prsm
typealias Position = Vector3
typealias EnemyList = List<Enemy>

val pos: Position = vec3(1, 2, 3)
val enemies: EnemyList = []
```

Aliases shall not form cycles (E126) and shall not shadow built-in types (E127).

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
