---
title: Declarations & Fields
parent: Language Guide
grand_parent: English Docs
nav_order: 5
---

# Declarations & Fields

## Top-level declarations

PrSM currently supports:

- `component`
- `asset`
- `class`
- `data class`
- `enum`
- `attribute`

Each file contains exactly one top-level declaration.

## Serialized fields

```prsm
@header("Movement")
serialize speed: Float = 5.0
```

This lowers to Unity-serializable C# while still exposing a property-facing API.

## Regular fields

- `val` for immutable fields and locals
- `var` for mutable fields and locals
- `public`, `private`, `protected` visibility modifiers

## Component lookup fields

Only `component` declarations support these forms:

- `require name: Type`
- `optional name: Type`
- `child name: Type`
- `parent name: Type`

These are assembled into generated `Awake()` logic during lowering.
