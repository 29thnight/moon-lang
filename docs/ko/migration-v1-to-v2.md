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

## PrSM 3 → PrSM 4

### 활성화

```toml
[language]
version = "4"
```

`version = "4"`는 30개 언어 4 기능을 모두 암묵적으로 활성화합니다. 전체 기능 플래그 목록은 [PrSM 4](spec/lang-4.md)에 문서화되어 있습니다.

### 새 기능

PrSM 4는 가장 큰 단일 언어 릴리스입니다. 네 가지 카테고리에 걸쳐 30개 기능을 추가합니다:

| 카테고리 | 주요 기능 |
|---------|----------|
| 문법 확장 | `try`/`catch`/`throw`, 함수 타입을 가진 람다, `static`/`const`, 컬렉션 리터럴 `[1,2,3]`/`{"k":v}`, `as`/`as!` 캐스팅, 프로퍼티 `get`/`set`, `extend Type {}`, `abstract`/`sealed`/`open`, 튜플, `use`, `struct`, `event`, 연산자 오버로딩, 인덱서, `typealias`, 원시 문자열 `"""..."""`, `in` 연산자, OR/범위 패턴, `?:=`, 디폴트 인터페이스 메서드 |
| 비동기 + 성능 | `async`/`await` (UniTask 우선), v4 옵티마이저 규칙 (`opt.string`, `opt.linq`, `opt.structcopy`), Burst 호환성 분석 |
| 패턴 편의 기능 | `state machine` 블록, `command` 선언, `bind` 반응형 프로퍼티 (MVVM) |
| 개발자 경험 | Rust/Elm 스타일 다중 줄 진단, LSP 리팩토링 코드 액션, 평탄한 `.prsm.map` 소스 맵 |

### Breaking changes

없음. 모든 언어 3 프로그램은 수정 없이 언어 4에서 컴파일됩니다. 새로운 contextual 키워드 (`async`, `await`, `event`, `use`, `state`, `machine`, `command`, `bind`, `to`, `enter`, `exit`, `on`, `get`, `set`, `field`)는 키워드 위치가 아닌 곳에서는 식별자로 여전히 유효합니다.

### 마이그레이션 절차

1. `.prsmproject`에 `version = "4"` 설정 (또는 **Window > PrSM > Project Settings** 사용)
2. `prism build` 실행 — 기존 언어 3 코드는 변경 없이 컴파일되어야 합니다
3. 기존 `intrinsic { try { … } catch (Exception e) { … } }` 블록을 네이티브 `try`/`catch`로 교체
4. 함수 전달용 `Action`/`Func` intrinsic 보일러플레이트를 람다식으로 교체
5. 새 패턴을 점진적으로 도입:
   - `new List<int>{1, 2, 3}` 대신 컬렉션 리터럴 (`[1, 2, 3]`) 사용
   - HUD component에 `bind` 반응형 프로퍼티 사용
   - 상태 머신 `enum` + `switch` 패턴을 `state machine` 블록으로 변환
   - 비동기 콘텐츠 로드에 `async`/`await` 추가
6. v4 옵티마이저 (`--optimize`)를 활성화하고 W026/W027 경고를 검토하여 핫 패스 할당 감사

### 롤백

`version = "3"`으로 변경하고 v4 전용 문법 제거 (`try`/`catch`, 람다, 컬렉션 리터럴, `state machine`, `command`, `bind` 등).

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
| `try-catch` | PrSM 4 | try/catch/finally 와 throw |
| `lambda` | PrSM 4 | 람다식과 함수 타입 |
| `static-const` | PrSM 4 | static 및 const 멤버 |
| `collection-literals` | PrSM 4 | List, Map, Set 리터럴 |
| `type-cast` | PrSM 4 | as / as! / 스마트 캐스트 |
| `property` | PrSM 4 | field를 가진 프로퍼티 get/set |
| `extend` | PrSM 4 | 확장 메서드 |
| `class-modifiers` | PrSM 4 | abstract / sealed / open |
| `tuple` | PrSM 4 | 튜플 타입과 구조 분해 |
| `use` | PrSM 4 | IDisposable을 위한 use |
| `struct` | PrSM 4 | struct 선언 |
| `event` | PrSM 4 | event 멀티캐스트 델리게이트 |
| `operator` | PrSM 4 | 연산자 오버로딩 |
| `indexer` | PrSM 4 | 인덱서 (operator get/set) |
| `typealias` | PrSM 4 | typealias 선언 |
| `raw-string` | PrSM 4 | 원시 문자열 리터럴 |
| `in-operator` | PrSM 4 | in 멤버십 연산자 |
| `or-pattern` | PrSM 4 | when의 OR 패턴 |
| `range-pattern` | PrSM 4 | when의 범위 패턴 |
| `null-coalesce-assign` | PrSM 4 | ?:= 연산자 |
| `default-interface` | PrSM 4 | 디폴트 인터페이스 메서드 |
| `async-await` | PrSM 4 | async / await |
| `optimizer-v4` | PrSM 4 | v4 옵티마이저 규칙 |
| `burst-analysis` | PrSM 4 | Burst 호환성 분석 |
| `state-machine` | PrSM 4 | state machine 편의 기능 |
| `command` | PrSM 4 | command 편의 기능 |
| `bind` | PrSM 4 | bind 반응형 프로퍼티 |
| `error-enhancement` | PrSM 4 | Rust/Elm 스타일 진단 |
| `refactor-tools` | PrSM 4 | LSP 리팩토링 코드 액션 |
| `debugger` | PrSM 4 | 평탄한 소스 맵 생성 |

`version = "N"` 설정 시 해당 버전 이하의 모든 기능이 암묵적으로 활성화됩니다.
