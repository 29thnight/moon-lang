---
title: Types
parent: Language Guide
grand_parent: English Docs
nav_order: 3
---

# Types

## Primitive types

| PrSM | C# |
|---|---|
| `Int` | `int` |
| `Float` | `float` |
| `Double` | `double` |
| `Bool` | `bool` |
| `String` | `string` |
| `Long` | `long` |
| `Byte` | `byte` |
| `Unit` | `void` |

## Unity and external types

Unity types such as `MonoBehaviour`, `ScriptableObject`, `Transform`, `Rigidbody`, `Animator`, `Vector2`, `Vector3`, and `Quaternion` are used directly and lowered directly into C#.

## Nullability

- `Type` means non-null
- `Type?` means nullable
- semantic analysis performs null-safety checks and limited smart-cast style narrowing for simple local cases

## Generic references

The current implementation supports generic type references and lowers them into .NET collection shapes where appropriate.
