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


## 제네릭 타입

PrSM 제네릭 타입은 .NET 대응 타입으로 lowering됩니다:

| PrSM | C# |
|---|---|
| `Array<T>` | `T[]` |
| `List<T>` | `System.Collections.Generic.List<T>` |
| `Map<K,V>` | `System.Collections.Generic.Dictionary<K,V>` |
| `Set<T>` | `System.Collections.Generic.HashSet<T>` |
| `Queue<T>` | `System.Collections.Generic.Queue<T>` |
| `Stack<T>` | `System.Collections.Generic.Stack<T>` |
| `Seq<T>` | `System.Collections.Generic.IEnumerable<T>` |

## 타입 추론

우변에서 타입이 명확할 때 로컬 변수 타입을 생략할 수 있습니다:

```prsm
val name = "Player"       // String으로 추론
val hp = 100              // Int로 추론
val speed = 5.0           // Float로 추론
var alive = true          // Bool로 추론
```

명시적 타입 표기는 항상 유효하며, 초기값이 `null`인 경우 필수입니다.

### 제네릭 타입 추론 (PrSM 2 부터)

v2는 제네릭 메서드 호출에 대한 제한적 문맥 기반 추론을 도입합니다. 자세한 내용은 [제네릭 추론](generic-inference.md)을 참조하세요.

```prsm
val rb: Rigidbody = get()        // 추론: GetComponent<Rigidbody>()
val health: Health? = child()    // 추론: GetComponentInChildren<Health>()
```

## Null 안전성 모델

PrSM은 타입 표기를 통해 컴파일 타임에 null 안전성을 강제합니다.

**Non-nullable 타입** (`Type`)은 값이 존재함을 보장합니다:

```prsm
require rb: Rigidbody     // Awake 이후 non-null 보장
val speed: Float = 5.0    // 절대 null이 아님
```

**Nullable 타입** (`Type?`)은 사용 전 가드가 필요합니다:

```prsm
optional cam: Camera?     // null일 수 있음

// 안전한 접근 패턴:
cam?.enabled = false          // 안전 호출 — null이면 무시
val depth = cam?.depth ?: 0   // elvis — 대체값
val fov = cam!!.fieldOfView   // non-null 단언 (이미 non-null이면 경고 W001)
```

## 전체 타입 매핑 참조

| PrSM | C# | 분류 |
|---|---|---|
| `Int` | `int` | 기본 타입 |
| `Float` | `float` | 기본 타입 |
| `Double` | `double` | 기본 타입 |
| `Bool` | `bool` | 기본 타입 |
| `String` | `string` | 기본 타입 |
| `Char` | `char` | 기본 타입 |
| `Long` | `long` | 기본 타입 |
| `Byte` | `byte` | 기본 타입 |
| `Unit` | `void` | 반환 타입 |
| `Array<T>` | `T[]` | 컬렉션 |
| `List<T>` | `System.Collections.Generic.List<T>` | 컬렉션 |
| `Map<K,V>` | `System.Collections.Generic.Dictionary<K,V>` | 컬렉션 |
| `Set<T>` | `System.Collections.Generic.HashSet<T>` | 컬렉션 |
| `Queue<T>` | `System.Collections.Generic.Queue<T>` | 컬렉션 |
| `Stack<T>` | `System.Collections.Generic.Stack<T>` | 컬렉션 |
| `Seq<T>` | `System.Collections.Generic.IEnumerable<T>` | 컬렉션 |
| *기타* | *변환 없이 그대로 전달* | Unity/.NET |
