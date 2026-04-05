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

## Type check

`is` tests whether a value is a given type:

```prsm
if collider is BoxCollider {
    handleBox()
}
```
