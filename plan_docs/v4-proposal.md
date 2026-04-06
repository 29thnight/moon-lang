# PrSM Language 4 제안서 — Draft

**Status:** 제안 단계 (미확정)
**선행 조건:** Language 3 (Prism v1.0.0) 릴리스 완료

---

## 1. v4 방향성

Language 3까지의 여정:
- v1: 핵심 문법 + Unity 통합
- v2: 패턴 매칭 + 이벤트 수명 + Input System
- v3: 인터페이스 + 제네릭 + 디자인 패턴 sugar + 정적 분석

**v4 목표: "실전 게임 개발에서 C#을 직접 쓸 이유가 남지 않게 한다"**

v3까지는 "C# 보일러플레이트 줄이기"에 집중했다면, v4는 "PrSM만으로 게임 전체를 만들 수 있는가?"에 답한다.

---

## 2. 후보 기능 전체 목록

### 카테고리 A: 패턴 언어화 (v3 확장선)

#### A1. `state machine` 블록

게임 개발에서 가장 자주 사용되는 패턴. 60줄+ → 15줄.

```prsm
component Enemy : MonoBehaviour {
    state machine {
        idle {
            enter { playAnim("idle") }
            execute { if canSeePlayer() { transition chase } }
        }
        chase {
            enter { playAnim("run") }
            execute {
                moveToward(player.position)
                if distanceTo(player) < attackRange { transition attack }
                if not canSeePlayer() { transition idle }
            }
        }
        attack {
            enter { playAnim("attack") }
            execute { if attackFinished { transition idle } }
        }
    }
}
```

생성: IState 인터페이스, 각 State 클래스, StateMachine 클래스, Enter/Execute/Exit, 초기 상태 설정.

**가치:** 높음 — 거의 모든 게임에 상태 머신 존재
**복잡도:** 높음 — AST 설계, 중첩 상태, 전환 조건 검증

#### A2. `command` 선언

실행 취소/재실행 시스템의 보일러플레이트 제거.

```prsm
command MoveCommand(player: PlayerMover, direction: Vector3) {
    execute { player.move(direction) }
    undo { player.move(-direction) }
}
```

생성: ICommand 인터페이스, Execute/Undo 메서드, CommandInvoker 스택 관리.

**가치:** 중간 — 에디터 도구, 턴제 게임에서 유용
**복잡도:** 중간

#### A3. `bind` 문법 (MVVM 데이터 바인딩)

UI와 데이터 모델의 자동 동기화.

```prsm
component HealthUI : MonoBehaviour {
    require player: PlayerHealth
    serialize healthBar: Slider
    serialize healthText: Text

    bind healthBar.value to player.hp / player.maxHp
    bind healthText.text to "${player.hp} / ${player.maxHp}"
}
```

생성: 프로퍼티 변경 감지 + UI 업데이트 코드. Unity UI Toolkit의 데이터 바인딩과 연계 가능.

**가치:** 높음 — UI 코드가 게임 코드의 30-40%
**복잡도:** 높음 — 양방향 바인딩, 변환 함수, 수명 관리

---

### 카테고리 B: 비동기 + 성능

#### B1. async/await (UniTask 통합)

코루틴의 한계를 넘는 비동기 프로그래밍.

```prsm
async func loadLevel(name: String) {
    showLoadingScreen()
    val scene = await SceneManager.LoadSceneAsync(name)
    await UniTask.delay(500)
    hideLoadingScreen()
}

async func fetchData(url: String): String {
    val request = await UnityWebRequest.Get(url).SendWebRequest()
    return request.downloadHandler.text
}
```

생성: UniTask 기반 비동기 메서드. `await`은 `await` 그대로, `async func`는 `async UniTask` 또는 `async UniTask<T>` 반환.

**가치:** 높음 — 씬 로딩, 네트워크, 파일 I/O에 필수
**복잡도:** 중간 — UniTask 패키지 의존성 감지 필요
**의존성:** Unity 프로젝트에 UniTask 패키지 설치 필요

#### B2. 옵티마이저 강화

v3에서 시작한 옵티마이저를 실용 수준으로 확장.

```
추가 규칙:
- GetComponent 루프 호이스팅 (Update 내 반복 호출 → Awake에서 캐싱)
- 문자열 연결 → StringBuilder 변환 (루프 내)
- LINQ → for 루프 변환 (IL2CPP 최적화)
- 불필요한 null 체크 제거 (require 필드는 non-null 보장)
- 상수 접기 확장 (산술 + 비교)
- dead code 제거 (도달 불가 분기)
```

**가치:** 중간 — 모바일/VR에서 성능 차이 체감
**복잡도:** 중간 — 각 규칙 독립 구현 가능

#### B3. Burst 호환성 분석

PrSM 코드가 Burst 컴파일 가능한지 정적 분석.

```
W020: Method 'calculatePath' uses managed types incompatible with Burst.
      Consider using NativeArray instead of List.
W021: Method 'processEntities' allocates on heap (new Vector3[]).
      Consider using stackalloc or NativeArray for Burst compatibility.
```

**가치:** 중간 — DOTS/Burst 사용자에게만 해당
**복잡도:** 높음 — Burst 제약 전체를 이해해야 함

---

### 카테고리 C: 개발자 경험 (DX)

#### C1. Hot Reload (연구)

.prsm 수정 → Play 모드 상태 유지하며 즉시 반영.

PrSM의 구조적 이점:
- AST diff로 변경 감지 (IL 비교보다 정확)
- 생성 C#이 순수 코드 → 네트워크 라이브러리 ILPostProcessor 충돌 없음
- 소스맵 이미 존재

**가치:** 매우 높음 — 게임 개발 이터레이션의 핵심
**복잡도:** 매우 높음 — IL 패칭 인프라가 별도 프로젝트급
**접근:** 단계적 — 먼저 "빠른 재컴파일" (도메인 재로드 최적화), 이후 "진짜 Hot Reload"

#### C2. 디버거 통합 (VS Code DAP)

PrSM 소스에서 직접 브레이크포인트 설정 + 변수 검사.

현재: 생성 C#에서 디버깅 → `#line` 디렉티브로 .prsm 위치 표시
목표: VS Code Debug Adapter Protocol로 .prsm 파일에서 직접 디버깅

```
.prsm 브레이크포인트 → 소스맵 → 생성 C# 위치 매핑 → Unity 디버거 연결
```

**가치:** 높음 — 현재 디버깅 경험의 큰 개선
**복잡도:** 높음 — DAP 어댑터 구현 필요

#### C3. 리팩토링 도구

LSP 기반 자동 리팩토링:
- **Extract Method**: 선택한 코드를 새 함수로 추출
- **Extract Component**: 컴포넌트의 일부를 새 컴포넌트로 분리 (SOLID SRP)
- **Inline Function**: 함수 본문을 호출 지점에 인라인
- **Convert to Interface**: 구체 타입 참조를 인터페이스로 변환

**가치:** 중간 — 대규모 프로젝트에서 유용
**복잡도:** 중간 — HIR 정보 활용

#### C4. 에러 메시지 개선

Rust/Elm 스타일의 친절한 에러 메시지.

```
error[E082]: Pattern binds 3 variable(s) but 'Action.Move' expects 1
  --> Assets/Enemy.prsm:15:7
   |
15 |     Action.Move(target, extra, more) => doMove(target)
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = help: 'Action.Move' has 1 parameter (target: String)
   = help: remove 'extra' and 'more' to match the enum definition
   |
   = note: enum Action(val target: String) defined at Assets/Actions.prsm:3
```

**가치:** 높음 — 학습 곡선 완화
**복잡도:** 중간 — 진단 포맷터 재작성

---

### 카테고리 D: 직렬화 + 데이터

#### D1. 직렬화 자동 생성

```prsm
@serializable
data class SaveData(
    playerName: String,
    level: Int,
    inventory: List<Item>,
    position: Vector3
)

// 자동 생성: ToJson(), FromJson(), ToBinary(), FromBinary()
val json = saveData.toJson()
val restored = SaveData.fromJson(json)
```

**가치:** 중간 — 세이브/로드 시스템에 유용
**복잡도:** 중간

#### D2. `config` 선언 (ScriptableObject 강화)

```prsm
config GameConfig {
    @range(1, 100)
    val maxEnemies: Int = 10

    @range(0.1, 10.0)
    val spawnRate: Float = 1.0

    val difficultyLevels: List<String> = ["Easy", "Normal", "Hard"]
}

// 자동 생성: ScriptableObject + 커스텀 에디터 + 검증
```

**가치:** 중간 — 게임 설정 관리에 유용
**복잡도:** 낮음 — 기존 asset의 확장

---

### 카테고리 E: 핵심 문법 확장

현재 PrSM에 없어서 `intrinsic`으로 빠질 수밖에 없는 기본 문법 요소들.

#### E1. try/catch/finally

현재 PrSM에서 예외 처리가 **전혀 불가능**. 모든 에러 처리가 intrinsic으로 빠짐.

```prsm
func loadConfig(path: String): Config {
    try {
        val data = File.readAllText(path)
        return Config.parse(data)
    } catch (e: FileNotFoundException) {
        log("Config not found, using defaults")
        return Config.default()
    } catch (e: Exception) {
        error("Failed to load config: ${e.message}")
        return Config.default()
    } finally {
        log("Config load attempted")
    }
}
```

생성 C#: 표준 try/catch/finally 그대로.

**가치:** 매우 높음 — 에러 처리는 모든 프로그램의 기본
**복잡도:** 낮음 — C#과 1:1 매핑

#### E2. lambda/클로저

현재 lambda는 `listen` 블록 내부에서만 암묵적으로 사용. 범용 lambda 표현식이 없음.

```prsm
// lambda 변수
val greet = { name: String => "Hello, $name" }
log(greet("PrSM"))

// 콜백 파라미터
func doAfterDelay(seconds: Float, action: () => Unit) {
    start delayedAction(seconds, action)
}

// LINQ 스타일
val alive = enemies.filter({ e => e.isAlive })
val names = players.map({ p => p.name })

// 후행 lambda (Kotlin 스타일)
enemies.filter { it.isAlive }
       .sortBy { it.distanceTo(player) }
       .take(3)
```

생성 C#: `System.Action`, `System.Func<>`, lambda 표현식.

**가치:** 매우 높음 — 이벤트 콜백, 컬렉션 처리, 비동기 완료 핸들러
**복잡도:** 중간 — 타입 추론이 복잡해질 수 있음 (클로저 캡처)

#### E3. static 멤버 + const

유틸리티 함수와 상수를 PrSM에서 정의할 수 없음.

```prsm
class MathUtils {
    const PI: Float = 3.14159
    const MAX_HEALTH: Int = 100

    static func clamp(value: Float, min: Float, max: Float): Float {
        if value < min { return min }
        if value > max { return max }
        return value
    }

    static func lerp(a: Float, b: Float, t: Float): Float = a + (b - a) * t
}

// 사용
val angle = MathUtils.PI * 2.0
val clamped = MathUtils.clamp(value, 0.0, 1.0)
```

생성 C#: `public const`, `public static` 그대로.

**가치:** 높음 — 모든 프로젝트에 유틸리티/상수 필요
**복잡도:** 낮음 — 키워드 추가 + lowering 직역

#### E4. 컬렉션 리터럴 + 타입 캐스팅

컬렉션 생성이 불필요하게 장황하고, 타입 변환 구문이 없음.

```prsm
// 컬렉션 리터럴
val numbers = [1, 2, 3, 4, 5]
val names = ["Alice", "Bob", "Charlie"]
val lookup = {"hp": 100, "mp": 50}        // Map 리터럴
val empty = <Int>[]                         // 빈 타입 리스트

// 타입 캐스팅
val collider = hit.collider
val enemy = collider as Enemy?             // 안전 캐스트 (실패 시 null)
val box = collider as! BoxCollider         // 강제 캐스트 (실패 시 예외)

// is + 스마트 캐스트 (기존 문법 확장)
if collider is BoxCollider {
    // collider가 자동으로 BoxCollider 타입으로 좁혀짐
    log(collider.size)
}
```

생성 C#:
- `[1, 2, 3]` → `new List<int> { 1, 2, 3 }`
- `{"a": 1}` → `new Dictionary<string, int> { {"a", 1} }`
- `as Enemy?` → `as Enemy`
- `as! BoxCollider` → `(BoxCollider)` (InvalidCastException 가능)
- is 스마트 캐스트 → C# pattern matching `if (collider is BoxCollider box)`

**가치:** 높음 — 일상적 코드의 간결함
**복잡도:** 중간 — 리터럴 타입 추론이 필요

---

## 3. 우선순위 평가

| # | 기능 | 가치 | 복잡도 | 가치/복잡도 | 카테고리 |
|---|------|------|--------|------------|---------|
| A1 | state machine | ★★★★★ | ★★★★ | 높음 | 패턴 |
| A3 | bind (MVVM) | ★★★★★ | ★★★★★ | 중간 | 패턴 |
| B1 | async/await | ★★★★★ | ★★★ | **매우 높음** | 비동기 |
| C4 | 에러 메시지 개선 | ★★★★ | ★★★ | 높음 | DX |
| B2 | 옵티마이저 강화 | ★★★ | ★★★ | 중간 | 성능 |
| A2 | command | ★★★ | ★★ | 높음 | 패턴 |
| D1 | 직렬화 | ★★★ | ★★★ | 중간 | 데이터 |
| C3 | 리팩토링 도구 | ★★★ | ★★★ | 중간 | DX |
| C2 | 디버거 통합 | ★★★★ | ★★★★★ | 중간 | DX |
| C1 | Hot Reload | ★★★★★ | ★★★★★★ | 낮음 | DX (연구) |
| B3 | Burst 분석 | ★★ | ★★★★ | 낮음 | 성능 |
| D2 | config 선언 | ★★ | ★ | 높음 | 데이터 |

---

## 4. 제안 v4 범위

### Tier 1 (핵심)

| # | 기능 | 유형 | 근거 |
|---|------|------|------|
| 1 | **try/catch/finally** | 문법 | 에러 처리 기본. 없으면 모든 예외 처리가 intrinsic으로 빠짐 |
| 2 | **lambda/클로저** | 문법 | `val onClick = { x => log(x) }`. 이벤트 콜백, LINQ, 범용 함수형 |
| 3 | **static 멤버 + const** | 문법 | 유틸리티 함수, 상수 정의. `static func`, `const val` |
| 4 | **컬렉션 리터럴 + 타입 캐스팅** | 문법 | `[1, 2, 3]` 초기화, `as` 캐스팅, 타입 변환 |
| 5 | **async/await** | 비동기 | 코루틴 한계를 넘는 비동기. UniTask 통합 |
| 6 | **state machine** | 패턴 | 게임 개발 핵심 패턴. 60줄→15줄 |
| 7 | **에러 메시지 개선** | DX | Rust/Elm 스타일 친절한 에러 + help 힌트 |
| 8 | **옵티마이저 강화** | 성능 | v3 인프라 위에 실용적 규칙 추가 |

### Tier 2 (고가치)

| # | 기능 | 유형 | 근거 |
|---|------|------|------|
| 9 | **command** | 패턴 | 에디터 도구/턴제 게임에 즉시 유용 |
| 10 | **bind (MVVM)** | 패턴 | UI 코드 비중이 크지만 UI Toolkit 안정화에 따라 |
| 11 | **직렬화** | 데이터 | 세이브/로드 보일러플레이트 제거 |

### Tier 3 (미래/연구)

| # | 기능 | 근거 |
|---|------|------|
| 8 | 디버거 통합 | DAP 어댑터 별도 프로젝트급 |
| 9 | 리팩토링 도구 | HIR 보강 후 가능 |
| 10 | Hot Reload | IL 패칭 인프라 별도 프로젝트급 |
| 11 | Burst 분석 | DOTS 사용자 한정 |

---

## 5. 개발 프로세스

v2.1에서 확정된 프로세스를 따른다:

1. **이 제안서 확정** → v4 범위 결정
2. `plan_docs/spec/v4-language-spec.md` — 언어 4 표준 초안 작성
3. 구현 (표준 문서 기준)
4. 문서 업데이트 (standard.md + lang-4.md + migration)
5. 릴리스 (Prism v2.0.0?)

---

## 6. 릴리스 네이밍

| 옵션 | 설명 |
|------|------|
| Prism v1.1.0 | Language 4지만 기존 코드 breaking change 없으면 minor |
| Prism v2.0.0 | Language 4 = 새로운 major. async/await은 패러다임 변화 |

→ async/await 포함 여부에 따라 결정. 포함하면 v2.0.0이 적절.
