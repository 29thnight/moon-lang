---
title: Types
parent: 언어 가이드
grand_parent: 한국어 문서
nav_order: 3
---

# Types

## 기본 타입

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

## Unity 및 외부 타입

`MonoBehaviour`, `ScriptableObject`, `Transform`, `Rigidbody`, `Animator`, `Vector2`, `Vector3`, `Quaternion` 같은 Unity 타입은 그대로 쓰고 그대로 C#으로 내려갑니다.

## Nullability

- `Type` 는 non-null
- `Type?` 는 nullable
- 의미 분석 단계에서 null-safety 검사와 단순 로컬 기준의 smart-cast 스타일 축소를 수행합니다

## 제네릭 참조

현재 구현은 제네릭 타입 표기를 인식하고, 필요한 경우 .NET 컬렉션 형태로 lowering 합니다.
