---
title: Strings & Collections
parent: 언어 가이드
grand_parent: 한국어 문서
nav_order: 4
---

# Strings & Collections

## 문자열 리터럴

문자열은 큰따옴표를 사용합니다:

```prsm
val greeting = "Hello, world!"
```

## 문자열 보간

PrSM은 문자열 리터럴 내부에서 두 가지 보간 형식을 지원합니다:

| 문법 | 용도 |
|---|---|
| `$name` | 단순 식별자 삽입 |
| `${expression}` | 임의 표현식 삽입 |

```prsm
func statusLine(): String = "HP: $hp / $maxHp"
func debugInfo(): String = "pos=${transform.position} vel=${rb.velocity.magnitude:.2}"
```

보간된 문자열은 C# 문자열 보간 (`$"..."`)으로 변환됩니다.

## 원시 문자열 리터럴 (PrSM 4 부터)

삼중 따옴표 문자열은 이스케이프 없이 줄바꿈과 특수 문자를 보존합니다. 선행 공백은 닫는 구분자의 들여쓰기 수준으로 트림됩니다. 보간(`$var`, `${expr}`)은 원시 문자열 내부에서도 활성 상태로 유지됩니다.

```prsm
val json = """
    {
        "name": "Player",
        "level": 42
    }
    """

val query = """
    SELECT * FROM users
    WHERE name = '${userName}'
    """
```

지원되는 곳에서는 C# 11 원시 문자열 리터럴로, 이전 타깃에서는 `@"..."` 축자 문자열로 변환됩니다. 종료되지 않은 원시 문자열은 E128을 발생시킵니다.

## 컬렉션

PrSM은 유용한 제네릭 컬렉션 타입 집합을 인식하며 .NET 대응 타입으로 변환합니다:

| PrSM | 생성 C# |
|---|---|
| `Array<T>` | `T[]` |
| `List<T>` | `System.Collections.Generic.List<T>` |
| `Map<K, V>` | `System.Collections.Generic.Dictionary<K, V>` |
| `Set<T>` | `System.Collections.Generic.HashSet<T>` |
| `Queue<T>` | `System.Collections.Generic.Queue<T>` |
| `Stack<T>` | `System.Collections.Generic.Stack<T>` |
| `Seq<T>` | `System.Collections.Generic.IEnumerable<T>` |

```prsm
var enemies: List<GameObject> = null
var scoreByPlayer: Map<String, Int> = null
var visitedRooms: Set<String> = null
```

컬렉션 필드는 C# 대응 타입과 동일한 방식으로 Unity 직렬화와 동작합니다. `serialize`로 표시된 `List<T>` 필드는 Inspector에 표시됩니다.

## 컬렉션 리터럴 (PrSM 4 부터)

PrSM 4는 `List<T>`, `Map<K, V>`, `Array<T>`, `Set<T>`을 위한 인라인 문법을 추가합니다:

```prsm
val numbers = [1, 2, 3, 4, 5]
val names = ["Alice", "Bob", "Charlie"]
val empty: List<Int> = []

val lookup = {"hp": 100, "mp": 50}
val arr: Array<Int> = arrayOf(1, 2, 3)
val unique = setOf("Alice", "Bob", "Alice")
```

생성 C#:

```csharp
var numbers = new List<int> { 1, 2, 3, 4, 5 };
var lookup = new Dictionary<string, int> { ["hp"] = 100, ["mp"] = 50 };
var arr = new int[] { 1, 2, 3 };
var unique = new HashSet<string> { "Alice", "Bob", "Alice" };
```

원소 타입은 첫 원소에서 추론됩니다. 빈 리터럴은 명시적 타입 주석이 필요합니다 (E107). 혼합된 원소 타입은 E108을 발생시킵니다.

## `in` 멤버십 연산자 (PrSM 4 부터)

`in`은 범위, 리스트, 맵에 대한 멤버십을 테스트합니다:

```prsm
if x in 1..10 { log("In range") }
if name in ["Alice", "Bob"] { log("Known user") }
if key in lookup { log("Key exists") }
```

생성 C#:

```csharp
if (x >= 1 && x <= 10) { Debug.Log("In range"); }
if (new List<string>{"Alice", "Bob"}.Contains(name)) { Debug.Log("Known user"); }
if (lookup.ContainsKey(key)) { Debug.Log("Key exists"); }
```

`Contains`도 `ContainsKey`도 가지지 않는 타입에 대한 `in`은 E129를 발생시킵니다.

## 컬렉션 다루기

컬렉션은 표준 .NET 타입으로 변환되므로, 모든 .NET 인스턴스 메서드를 직접 호출할 수 있습니다:

```prsm
enemies.Add(newEnemy)
enemies.Remove(deadEnemy)
val count = enemies.Count
```
