---
title: Operators
parent: Language Guide
grand_parent: English Docs
nav_order: 2
---

# Operators

## Arithmetic

| Operator | Description |
|---|---|
| `+` | addition |
| `-` | subtraction |
| `*` | multiplication |
| `/` | division |
| `%` | modulo |

```prsm
val damage = baseDamage * multiplier
val remaining = maxHp - hp
```

## Comparison

| Operator | Description |
|---|---|
| `==` | equal |
| `!=` | not equal |
| `<` | less than |
| `>` | greater than |
| `<=` | less than or equal |
| `>=` | greater than or equal |

## Logical

| Operator | Description |
|---|---|
| `&&` | logical and |
| `\|\|` | logical or |
| `!` | logical not |

## Assignment

| Operator | Description |
|---|---|
| `=` | assignment |
| `+=` | add-assign |
| `-=` | subtract-assign |
| `*=` | multiply-assign |
| `/=` | divide-assign |
| `%=` | modulo-assign |
| `?:=` (since PrSM 4) | null coalescing assign — assigns only when the left side is `null` |

```prsm
var _instance: GameManager? = null

func getInstance(): GameManager {
    _instance ?:= FindFirstObjectByType<GameManager>()
    return _instance!!
}
```

`_instance ?:= expr` lowers to `_instance ??= expr`. The left-hand side must be a nullable mutable variable; otherwise the compiler raises E132 (non-nullable) or E133 (`val`).

## Null-safety

| Operator | Description |
|---|---|
| `?.` | safe member access — short-circuits on null |
| `?:` | null-coalescing (Elvis) — fallback value when null |
| `!!` | non-null assertion — throws if null |

```prsm
val name = player?.name ?: "Unknown"
val rb = body!!
```

## Range and loop operators

| Operator | Description |
|---|---|
| `..` | inclusive range |
| `until` | exclusive upper bound |
| `downTo` | descending range |
| `step` | range step size |

```prsm
for i in 0 until count { tick(i) }
for i in 10 downTo 0 step 2 { countdown(i) }
```

## Type check and casting

`is` tests whether a value is a given type:

```prsm
if collider is BoxCollider {
    handleBox()
}
```

After an `is` check, the variable is smart-cast to the checked type within the same scope (since PrSM 4).

### Cast operators (since PrSM 4)

| Operator | Description |
|---|---|
| `as Type?` | safe cast — returns `null` on failure |
| `as! Type` | force cast — throws `InvalidCastException` on failure |

```prsm
val enemy = collider as Enemy?      // Enemy or null
val boss = collider as! Boss        // throws on mismatch
```

`as!` to a provably unrelated type produces E109. `as?` results that are never null-checked emit W021.

## `in` membership operator (since PrSM 4)

`in` tests membership against ranges, lists, and maps:

```prsm
if x in 1..10 { log("In range") }
if name in ["Alice", "Bob"] { log("Known user") }
if key in lookup { log("Key exists") }
```

A type without `Contains` or `ContainsKey` produces E129.

## `await` (since PrSM 4)

`await` is a prefix operator inside `async func` bodies that suspends until the awaited task completes:

```prsm
async func loadData(url: String): String {
    val response = await Http.get(url)
    return response.body
}
```

Using `await` outside an `async func` produces E135.

## Operator overloading (since PrSM 4)

Custom types may define operator functions. PrSM follows Kotlin conventions:

| Operator name | Symbol |
|---|---|
| `plus` | `+` |
| `minus` | `-` |
| `times` | `*` |
| `div` | `/` |
| `mod` | `%` |
| `compareTo` | `<` `>` `<=` `>=` |
| `equals` | `==` `!=` |
| `unaryMinus` | `-` (prefix) |
| `not` | `!` |

```prsm
data class Vec2i(x: Int, y: Int) {
    operator plus(other: Vec2i): Vec2i = Vec2i(x + other.x, y + other.y)
}

val c = Vec2i(1, 2) + Vec2i(3, 4)
```

`operator equals` requires a matching `GetHashCode` override (E124).
