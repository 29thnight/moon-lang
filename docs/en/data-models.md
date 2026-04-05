---
title: Data Models & Attributes
parent: Language Guide
grand_parent: English Docs
nav_order: 6
---

# Data Models & Attributes

## Data class

PrSM does not currently expose a `struct` keyword. The implemented data-model feature is `data class`.

```prsm
data class DamageInfo(
    val amount: Int,
    val crit: Bool
)
```

This lowers to a serializable C# class with public fields, a constructor, `Equals`, `GetHashCode`, and `ToString`.

## Enum

```prsm
enum EnemyState {
    Idle,
    Chase,
    Attack
}
```

Parameterized enums are also supported and lower to a normal enum plus generated extension methods for payload access.

## Attribute

```prsm
@targets(Method, Property)
attribute Cooldown(
    val duration: Float,
    val resetOnHit: Bool
)
```

This lowers to a C# attribute class with generated constructor and `AttributeUsage` metadata.
