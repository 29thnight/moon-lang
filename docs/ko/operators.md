---
title: Operators
parent: 언어 가이드
grand_parent: 한국어 문서
nav_order: 2
---

# Operators

## 산술

| 연산자 | 설명 |
|---|---|
| `+` | 덧셈 |
| `-` | 뺄셈 |
| `*` | 곱셈 |
| `/` | 나눗셈 |
| `%` | 나머지 |

```prsm
val damage = baseDamage * multiplier
val remaining = maxHp - hp
```

## 비교

| 연산자 | 설명 |
|---|---|
| `==` | 같음 |
| `!=` | 다름 |
| `<` | 미만 |
| `>` | 초과 |
| `<=` | 이하 |
| `>=` | 이상 |

## 논리

| 연산자 | 설명 |
|---|---|
| `&&` | 논리 AND |
| `\|\|` | 논리 OR |
| `!` | 논리 NOT |

## 대입

| 연산자 | 설명 |
|---|---|
| `=` | 대입 |
| `+=` | 더하고 대입 |
| `-=` | 빼고 대입 |
| `*=` | 곱하고 대입 |
| `/=` | 나누고 대입 |
| `%=` | 나머지 대입 |
| `?:=` (PrSM 4 부터) | Null 병합 대입 — 좌변이 `null`인 경우에만 대입 |

```prsm
var _instance: GameManager? = null

func getInstance(): GameManager {
    _instance ?:= FindFirstObjectByType<GameManager>()
    return _instance!!
}
```

`_instance ?:= expr`은 `_instance ??= expr`로 변환됩니다. 좌변은 nullable 가변 변수여야 합니다. 그렇지 않으면 컴파일러가 E132 (non-nullable) 또는 E133 (`val`)을 발생시킵니다.

## Null 안전성

| 연산자 | 설명 |
|---|---|
| `?.` | 안전 멤버 접근 — null이면 단락 |
| `?:` | null 병합 (Elvis) — null일 때 대체값 |
| `!!` | non-null 단언 — null이면 예외 |

```prsm
val name = player?.name ?: "Unknown"
val rb = body!!
```

## 범위와 루프 연산자

| 연산자 | 설명 |
|---|---|
| `..` | 닫힌 범위 |
| `until` | 배타적 상한 |
| `downTo` | 내림차순 범위 |
| `step` | 범위 단계 크기 |

```prsm
for i in 0 until count { tick(i) }
for i in 10 downTo 0 step 2 { countdown(i) }
```

## 타입 검사 및 캐스팅

`is`는 값이 주어진 타입인지 테스트합니다:

```prsm
if collider is BoxCollider {
    handleBox()
}
```

`is` 검사 후 변수는 같은 스코프 내에서 검사된 타입으로 스마트 캐스트됩니다 (PrSM 4 부터).

### 캐스트 연산자 (PrSM 4 부터)

| 연산자 | 설명 |
|---|---|
| `as Type?` | 안전 캐스트 — 실패 시 `null` 반환 |
| `as! Type` | 강제 캐스트 — 실패 시 `InvalidCastException` 발생 |

```prsm
val enemy = collider as Enemy?      // Enemy 또는 null
val boss = collider as! Boss        // 불일치 시 예외
```

명백히 무관한 타입으로의 `as!`는 E109를 발생시킵니다. null 검사되지 않는 `as?` 결과는 W021을 발생시킵니다.

## `in` 멤버십 연산자 (PrSM 4 부터)

`in`은 범위, 리스트, 맵에 대한 멤버십을 테스트합니다:

```prsm
if x in 1..10 { log("In range") }
if name in ["Alice", "Bob"] { log("Known user") }
if key in lookup { log("Key exists") }
```

`Contains`도 `ContainsKey`도 가지지 않는 타입은 E129를 발생시킵니다.

## `await` (PrSM 4 부터)

`await`는 `async func` 본문 내의 prefix 연산자로, awaited 태스크가 완료될 때까지 일시 중단합니다:

```prsm
async func loadData(url: String): String {
    val response = await Http.get(url)
    return response.body
}
```

`async func` 외부에서 `await`를 사용하면 E135가 발생합니다.

## 연산자 오버로딩 (PrSM 4 부터)

사용자 정의 타입은 연산자 함수를 정의할 수 있습니다. PrSM은 Kotlin 규칙을 따릅니다:

| 연산자 이름 | 기호 |
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

`operator equals`는 일치하는 `GetHashCode` 오버라이드를 요구합니다 (E124).
