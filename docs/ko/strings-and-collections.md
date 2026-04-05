---
title: Strings & Collections
parent: 언어 가이드
grand_parent: 한국어 문서
nav_order: 4
---

# Strings & Collections

## 문자열

PrSM은 일반 문자열 리터럴과 문자열 보간을 지원합니다.

```prsm
serialize name: String = "test"
func label(): String = "HP: $hp / ${maxHp}"
```

보간 형식:

- `$name`
- `${expression}`

## 컬렉션과 배열형 타입

PrSM은 별도의 표준 라이브러리 문서를 아직 갖고 있지는 않지만, 다음과 같은 컬렉션 타입 이름은 이미 이해합니다.

- `Array<T>` -> `T[]`
- `List<T>` -> `System.Collections.Generic.List<T>`
- `Map<K, V>` -> `System.Collections.Generic.Dictionary<K, V>`
- `Set<T>` -> `System.Collections.Generic.HashSet<T>`
- `Queue<T>`, `Stack<T>` 인식
- `Seq<T>` -> `System.Collections.Generic.IEnumerable<T>`

예시:

```prsm
var items: List<Int> = null
var lookup: Map<String, Int> = null
var tags: Set<String> = null
```
