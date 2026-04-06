---
title: 버전별 마이그레이션
parent: 언어 가이드
grand_parent: 한국어 문서
nav_order: 13
---

# 버전별 마이그레이션

PrSM 언어 버전 간 마이그레이션 가이드입니다. 각 버전은 완전한 하위 호환성을 유지합니다 — 이전 버전의 모든 프로그램은 변경 없이 컴파일됩니다.

---

## PrSM 1 → PrSM 2

### 활성화

```toml
[language]
version = "2"
features = ["pattern-bindings", "input-system", "auto-unlisten"]
```

### 새 기능

| 기능 | 설명 |
|------|------|
| 패턴 매칭과 바인딩 | `when` 분기에서 `EnemyState.Chase(target) =>` |
| listen 수명 모델 | `until disable`, `until destroy`, `manual` + `unlisten` |
| 구조 분해 | `val Stats(hp, speed) = getStats()` |
| New Input System sugar | `input.action("Jump").pressed` |
| 제네릭 타입 추론 | `val rb: Rigidbody = get()` |

### Breaking changes

없음. 수정자 없는 `listen`은 변경 없음 (등록만). 패턴 바인딩이 이제 검증됨 — 이전에 검사되지 않았던 패턴에서 E081/E082 발생 가능.

### 마이그레이션 절차

1. `.prsmproject`에 `version = "2"` 설정
2. `features` 배열에 원하는 기능 추가
3. `prism build` 실행 — E081/E082/E083 진단 수정
4. 새 기능을 점진적으로 도입:
   - 수명이 긴 listen 문에 `until disable` 추가
   - 수동 정리 intrinsic 블록을 `unlisten`으로 교체
   - 레거시 `input.getKey()` 대신 `input.action()` 사용

### 롤백

`version = "1"`로 변경하고 v2 전용 문법 제거 (`until disable`, `unlisten`, 패턴 바인딩).

---

## PrSM 2 → PrSM 3

### 활성화

```toml
[language]
version = "3"
features = ["pattern-bindings", "input-system", "auto-unlisten", "interface", "generics", "singleton", "pool", "solid-analysis", "optimizer"]
```

또는 Unity 에디터 GUI 사용: **Window > PrSM > Project Settings**

### 새 기능

| 기능 | 설명 |
|------|------|
| 인터페이스 선언 | `interface IDamageable { func takeDamage(amount: Int) }` |
| 제네릭 선언 | `class Registry<T> where T : Component { }` |
| `singleton` 키워드 | `singleton component GameManager : MonoBehaviour { }` |
| `pool` 수식자 | `pool bullets: Bullet(capacity = 20, max = 100)` |
| SOLID 분석 | W010/W011/W012 설계 문제 경고 |
| 코드 옵티마이저 | 단일 바인딩 구조 분해 인라인 |
| 예약어 | `get`, `find`가 내장 메서드 예약어 (E101) |

### Breaking changes

**E101 — 예약된 메서드 이름:** `get` 또는 `find`라는 이름의 사용자 정의 함수가 있다면 이름을 변경해야 합니다. 이 이름들은 내장 sugar 메서드(`GetComponent`, `FindFirstObjectByType`)를 위해 예약되어 있습니다.

```prsm
// Before (v2) — 컴파일되지만 sugar와 충돌 가능
func get(): Item { return items[0] }

// After (v3) — E101 방지를 위해 이름 변경
func getItem(): Item { return items[0] }
```

### 마이그레이션 절차

1. `.prsmproject`에 `version = "3"` 설정 (또는 **Window > PrSM > Project Settings** 사용)
2. `prism build` 실행 — E101 진단 수정 (`get`/`find` 함수 이름 변경)
3. SOLID 경고(W010/W011/W012) 검토 후 필요 시 리팩토링
4. 새 기능을 점진적으로 도입:
   - 컴포넌트 계약을 위한 인터페이스 정의
   - 수동 싱글톤 보일러플레이트 대신 `singleton` 사용
   - 수동 오브젝트 풀 관리 대신 `pool` 사용
   - 유틸리티 클래스에 제네릭 추가

### 롤백

`version = "2"`로 변경하고 v3 전용 문법 제거 (`interface`, 제네릭 파라미터, `singleton`, `pool`).

---

## Feature flag 레퍼런스

| 플래그 | 도입 버전 | 설명 |
|--------|----------|------|
| `pattern-bindings` | PrSM 2 | Enum payload 바인딩, 구조 분해, when 가드 |
| `input-system` | PrSM 2 | Input System sugar (Unity Input System 패키지 필요) |
| `auto-unlisten` | PrSM 2 | listen 수명 수정자 및 unlisten |
| `interface` | PrSM 3 | 인터페이스 선언 |
| `generics` | PrSM 3 | 제네릭 class/func + where 제약 |
| `singleton` | PrSM 3 | 싱글톤 컴포넌트 키워드 |
| `pool` | PrSM 3 | 오브젝트 풀 수식자 |
| `solid-analysis` | PrSM 3 | SOLID 분석 경고 |
| `optimizer` | PrSM 3 | 코드 옵티마이저 |

`version = "N"` 설정 시 해당 버전 이하의 모든 기능이 암묵적으로 활성화됩니다.
