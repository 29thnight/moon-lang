# PrSM v3 분석 — 코드 최적화 + 디자인 패턴 언어 지원

## 1. PDF 분석 요약

Unity 공식 가이드 "Level Up Your Code with Design Patterns" (148페이지)에서 다루는 패턴:

| 패턴 | C# 보일러플레이트 | PrSM v3 언어 지원 가능성 |
|------|-------------------|----------------------|
| **싱글톤** | 30줄+ (Instance, Awake 중복 체크, DontDestroyOnLoad, 제네릭 상속) | **높음** — `singleton` 키워드로 1줄 |
| **관찰자/이벤트** | UnityEvent/Action 선언, AddListener/RemoveListener | **이미 구현** — `listen` 문법 |
| **상태 머신** | IState 인터페이스, StateMachine 클래스, Enter/Execute/Exit, 전환 로직 (60줄+) | **높음** — `state` 블록 문법 |
| **커맨드** | ICommand 인터페이스, Execute/Undo, CommandInvoker, 스택 관리 (40줄+) | **중간** — `command` 문법 가능 |
| **팩토리** | IProduct 인터페이스, abstract Factory, Instantiate + GetComponent (30줄+) | **중간** — `factory` 문법 가능 |
| **오브젝트 풀** | Pool 클래스, Stack, Get/Return, 콜백 4개 (50줄+) | **중간** — `pool` 수식자 가능 |
| **MVP/MVVM** | Model/View/Presenter 분리, 바인딩, 이벤트 전파 (80줄+) | **높음** — `bind` 문법 |
| **전략** | 인터페이스 + 여러 구현 클래스 + 런타임 교체 | **낮음** — 기존 인터페이스로 충분 |
| **플라이웨이트** | ScriptableObject 기반 공유 데이터 | **낮음** — `asset` 문법으로 충분 |
| **더티 플래그** | 변경 추적 bool + 조건부 업데이트 | **중간** — `tracked` 수식자 가능 |

---

## 2. 사용자 요청 분석

### 2.1 코드 옵티마이저 (C# 자동 최적화)

현재 lowering은 "읽기 쉬운 C#"을 우선합니다. v3에서 최적화 패스를 추가하면:

**실현 가능한 최적화:**
- **불필요한 임시 변수 제거**: `var _prsm_d = expr; var a = _prsm_d.a;` → `var a = expr.a;` (단일 사용 시)
- **문자열 보간 최적화**: 상수 보간 → 컴파일 타임 연결
- **GetComponent 캐싱**: 같은 타입 반복 호출 감지 → 필드로 승격
- **박싱 회피**: `List<int>` foreach → for 루프 변환 (IL2CPP 최적화)
- **널 체크 병합**: 연속 `?.` 체인 → 단일 null 체크
- **Burst 호환 코드 생성**: `[BurstCompile]` 가능 여부 분석 + 자동 어노테이션

**실현 어려운 최적화 (v3 범위 밖):**
- 전역 흐름 분석 기반 최적화 (전체 프로젝트 의존성 필요)
- 런타임 성능 프로파일링 기반 최적화

### 2.2 디자인 패턴 보일러플레이트 제거

PDF의 패턴을 분석한 결과, PrSM이 언어 수준에서 제거할 수 있는 보일러플레이트:

#### A. 싱글톤 (30줄 → 1줄)

```prsm
// v3
singleton component GameManager : MonoBehaviour {
    var score: Int = 0
}
// 생성: static Instance, Awake 중복 체크, DontDestroyOnLoad 전부 자동
```

#### B. 상태 머신 (60줄+ → 15줄)

```prsm
// v3
component PlayerController : MonoBehaviour {
    state machine {
        idle {
            enter { playAnim("idle") }
            execute { if input.action("Move").held { transition walk } }
            exit { }
        }
        walk {
            enter { playAnim("walk") }
            execute {
                move(input.action("Move").vector2)
                if input.action("Jump").pressed { transition jump }
            }
        }
        jump {
            enter { rb.addForce(vec3(0, jumpForce, 0)) }
            execute { if isGrounded { transition idle } }
        }
    }
}
// 생성: IState 인터페이스, StateMachine 클래스, 각 State 클래스, Enter/Execute/Exit 메서드 전부 자동
```

#### C. 커맨드 (40줄+ → 선언형)

```prsm
// v3
command MoveCommand(player: PlayerMover, movement: Vector3) {
    execute { player.move(movement) }
    undo { player.move(-movement) }
}

// 사용
val cmd = MoveCommand(player, vec3(1, 0, 0))
cmd.execute()
cmd.undo()
```

#### D. 오브젝트 풀 (50줄+ → 수식자)

```prsm
// v3
component BulletSpawner : MonoBehaviour {
    pool bullets: Bullet(capacity = 20, max = 100)

    func fire() {
        val bullet = bullets.get()
        bullet.launch(direction)
    }
}
```

#### E. 데이터 바인딩/MVVM (80줄+ → bind 문법)

```prsm
// v3
component HealthUI : MonoBehaviour {
    bind healthBar.value to player.health / player.maxHealth
    bind healthText.text to "${player.health} / ${player.maxHealth}"
}
```

---

## 3. 추가 제안 사항 (PDF 외)

### 3.1 인터페이스 선언

현재 PrSM은 인터페이스 정의를 지원하지 않음 (C# 인터페이스를 직접 참조만 가능). v3에서:

```prsm
interface IDamageable {
    func takeDamage(amount: Int)
    val isAlive: Bool
}

component Enemy : MonoBehaviour, IDamageable {
    var hp: Int = 100
    val isAlive: Bool = hp > 0
    func takeDamage(amount: Int) { hp -= amount }
}
```

### 3.2 제네릭 선언

현재 제네릭 타입을 사용은 가능하지만 정의할 수 없음:

```prsm
class Pool<T>(capacity: Int) where T : MonoBehaviour {
    // ...
}
```

### 3.3 async/await (UniTask 통합)

Unity 코루틴의 한계를 넘는 비동기:

```prsm
async func loadLevel(name: String) {
    val scene = await SceneManager.LoadSceneAsync(name)
    await UniTask.delay(1000)
    fadeIn()
}
```

### 3.4 ECS 경량 지원

Unity DOTS와의 연결:

```prsm
system MoveSystem : SystemBase {
    query entities with(Translation, Velocity) {
        translation.value += velocity.value * Time.deltaTime
    }
}
```

### 3.5 시리얼라이제이션 강화

```prsm
@serializable
data class SaveData(
    playerName: String,
    level: Int,
    inventory: List<Item>
)
// 자동으로 JSON/Binary 직렬화 코드 생성
```

### 3.6 SOLID 원칙 강제

PDF에서 강조하는 SOLID를 언어 수준에서:

- **단일 책임**: 컴파일러가 컴포넌트의 책임 수를 분석하고 경고 (W010: "이 컴포넌트는 5개 이상의 독립적 관심사를 가집니다")
- **개방-폐쇄**: `sealed` 키워드로 확장 금지 명시, `open` 키워드로 확장 허용
- **종속성 역전**: `require` 에 인터페이스 타입 사용 시 자동 DI

---

## 4. v3 범위 (확정)

### v3 핵심 기능 (6개)

| # | 기능 | 유형 | 효과 |
|---|------|------|------|
| 1 | **인터페이스 선언** | 언어 확장 | PrSM 내에서 `interface` 정의 가능. 현재는 C# 인터페이스 참조만 가능 |
| 2 | **제네릭 선언** | 언어 확장 | `class<T>`, `func<T>` 정의 가능. 타입 제약 (`where T : MonoBehaviour`) 포함 |
| 3 | **C# 코드 옵티마이저** | 컴파일러 | GetComponent 캐싱, 임시 변수 제거, 널 체크 병합, 박싱 회피 |
| 4 | **`singleton` 키워드** | 패턴 sugar | 싱글톤 보일러플레이트 30줄 → `singleton component Name` 1줄 |
| 5 | **`pool` 수식자** | 패턴 sugar | 오브젝트 풀 50줄 → `pool name: Type(capacity)` 2줄 |
| 6 | **SOLID 분석 경고** | 정적 분석 | 단일 책임 위반 감지(W010), 과도한 의존성 경고(W011) |

### v3 각 기능 상세

#### 1. 인터페이스 선언

```prsm
interface IDamageable {
    func takeDamage(amount: Int)
    val isAlive: Bool
}

component Enemy : MonoBehaviour, IDamageable {
    var hp: Int = 100
    val isAlive: Bool = hp > 0
    func takeDamage(amount: Int) { hp -= amount }
}
```

생성 C#: 표준 C# `interface` + 구현 클래스. 컴파일러가 인터페이스 멤버 구현 여부를 검증.

#### 2. 제네릭 선언

```prsm
class ObjectPool<T>(capacity: Int) where T : MonoBehaviour {
    var items: List<T> = null
    func get(): T { /* ... */ }
    func release(item: T) { /* ... */ }
}

// 사용
val pool = ObjectPool<Bullet>(20)
val bullet = pool.get()
```

생성 C#: 표준 C# 제네릭 클래스. 타입 제약은 `where` 절로 전달.
제네릭 함수도 지원:

```prsm
func findAll<T>(): List<T> where T : Component {
    return FindObjectsByType<T>(FindObjectsSortMode.None).toList()
}
```

#### 3. C# 코드 옵티마이저

lowering과 codegen 사이에 최적화 패스를 삽입한다.

```
Parser → Semantic → HIR → Lowering → [Optimizer] → CodeGen
```

최적화 규칙:

| 규칙 | Before | After |
|------|--------|-------|
| GetComponent 캐싱 | 루프 내 `GetComponent<T>()` 반복 호출 | 메서드 시작에 로컬 변수로 캐싱 |
| 임시 변수 제거 | `var _prsm_d = expr; var a = _prsm_d.a;` (단일 사용) | `var a = expr.a;` |
| 널 체크 병합 | `if (a != null) { if (a.b != null) { a.b.c(); } }` | `a?.b?.c();` |
| 박싱 회피 | `foreach (var item in list)` (값 타입 리스트) | `for (int i = 0; ...)` |
| 상수 문자열 접기 | `$"hello {"world"}"` | `"hello world"` |
| 불필요한 캐스트 제거 | `(Type)GetComponent<Type>()` | `GetComponent<Type>()` |

옵티마이저는 `--optimize` 플래그로 활성화. 기본값은 비활성 (읽기 쉬운 C# 우선).

#### 4. `singleton` 키워드

```prsm
singleton component GameManager : MonoBehaviour {
    var score: Int = 0
    var isGameOver: Bool = false

    func addScore(amount: Int) {
        score += amount
    }
}

// 다른 곳에서 사용
GameManager.instance.addScore(100)
```

생성 C#:

```csharp
public class GameManager : MonoBehaviour
{
    private static GameManager _instance;
    public static GameManager Instance
    {
        get
        {
            if (_instance == null)
            {
                _instance = FindFirstObjectByType<GameManager>();
                if (_instance == null)
                {
                    var go = new GameObject("GameManager");
                    _instance = go.AddComponent<GameManager>();
                }
            }
            return _instance;
        }
    }

    private void Awake()
    {
        if (_instance == null)
        {
            _instance = this;
            DontDestroyOnLoad(gameObject);
        }
        else if (_instance != this)
        {
            Destroy(gameObject);
        }
    }

    // ... 사용자 멤버
}
```

옵션:
- `singleton component Name` — DontDestroyOnLoad 포함 (기본값)
- `singleton component Name : scoped` — 씬 한정 싱글톤 (DontDestroyOnLoad 없음)

#### 5. `pool` 수식자

```prsm
component BulletSpawner : MonoBehaviour {
    pool bullets: Bullet(capacity = 20, max = 100)

    func fire(direction: Vector3) {
        val bullet = bullets.get()
        bullet.transform.position = transform.position
        bullet.launch(direction)
    }
}

component Bullet : MonoBehaviour {
    func launch(direction: Vector3) { /* ... */ }

    func onHit() {
        bullets.release(this)  // 또는 pool.release(this)
    }
}
```

생성 C#: `UnityEngine.Pool.ObjectPool<T>` 기반 코드. CreateFunc, OnGet, OnRelease, OnDestroy 콜백 자동 생성.

#### 6. SOLID 분석 경고

시맨틱 분석에 정적 분석 패스를 추가한다.

| 코드 | 조건 | 설명 |
|------|------|------|
| W010 | 컴포넌트의 public 메서드가 8개+ | "이 컴포넌트는 많은 책임을 가지고 있습니다. 분리를 고려하세요." |
| W011 | require/optional 필드가 6개+ | "이 컴포넌트의 의존성이 많습니다. 종속성 역전을 고려하세요." |
| W012 | 단일 메서드가 50줄+ | "이 메서드가 길어졌습니다. 분리를 고려하세요." |
| W013 | require에 구체 타입 사용 시 (인터페이스 아닌 경우) | "인터페이스 타입을 사용하면 종속성 역전 원칙에 부합합니다." (인터페이스 선언 기능과 연동) |

모든 SOLID 경고는 `.prsmproject`에서 비활성화 가능:

```toml
[analysis]
solid_warnings = false  # 또는 개별: disabled_warnings = ["W010", "W012"]
```

### 미래 (v3 이후)

| 기능 | 비고 |
|------|------|
| `state machine` 블록 | 상태 머신 sugar — 설계 복잡도 높음, v4 후보 |
| `command` 선언 | 커맨드 패턴 sugar — v4 후보 |
| `bind` 문법 | MVVM 데이터 바인딩 — Unity UI Toolkit 안정화 후 |
| async/await | UniTask 통합 — Unity 공식 지원 상태에 따라 |
| 직렬화 자동 생성 | JSON/Binary 시리얼라이저 — v4 후보 |
| Hot Reload (연구) | .prsm 수정 → Play 모드 상태 유지하며 반영. PrSM이 AST diff로 변경 감지 가능하여 IL 비교보다 정확. 네트워크 라이브러리 충돌 없음 (순수 C# 생성). 다만 IL 패칭/DynamicMethod 인프라 구축이 별도 프로젝트급이므로 v4 이후 연구 과제 |

### 제외 (영구)

| 기능 | 사유 |
|------|------|
| ECS/DOTS 문법 | PrSM의 근본 전제(MonoBehaviour C# 생성)와 호환 불가. ECS는 별도 백엔드가 필요하며 PrSM의 정체성과 맞지 않음. intrinsic으로 대체. |
| 매크로/메타프로그래밍 | v0부터 비목표로 설정. 컴파일 타임 코드 생성은 컴파일러의 역할 |
| 커스텀 VM/런타임 | v0부터 비목표. 모든 실행은 Unity Mono/IL2CPP 경유 |

---

## 5. 개발 프로세스 (v2.1에서 확정)

**표준 문서 먼저 → 구현**

1. `plan_docs/spec/v3-language-spec.md` — 언어 3 표준 초안
2. 커뮤니티/내부 리뷰
3. 컴파일러 구현
4. 문서 업데이트 (`docs/en/spec/standard.md` 갱신 + `changes-lang-3.md`)
5. 릴리스 (Prism v1.0.0 = 언어 3)
