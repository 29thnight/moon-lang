# PrSM Language 3 Specification — Draft

**Status:** Draft v0.1
**Date:** 2026-04-06
**Prerequisite:** Language 2 (PrSM Language Standard)
**Target:** Unity 2022.3+ (IL2CPP / Mono)
**Tool version:** Prism v1.0.0 (Language 3 = Prism 1.0)

---

이 문서는 Language 3에서 추가되는 6개 기능의 표준 초안이다. Language 2의 모든 기능은 유지되며, 이 문서는 추가/변경 사항만 정의한다. 전체 언어 사양은 Language 3 확정 시 `docs/en/spec/standard.md`에 통합한다.

---

## 1. 인터페이스 선언 [decl.interface]

### 1.1 문법

```ebnf
InterfaceDecl = "interface" Identifier [ InterfaceExtends ] "{" { InterfaceMember } "}"

InterfaceExtends = ":" Identifier { "," Identifier }

InterfaceMember = InterfaceFunc | InterfaceProperty

InterfaceFunc = "func" Identifier "(" [ ParamList ] ")" [ ":" TypeRef ]

InterfaceProperty = ( "val" | "var" ) Identifier ":" TypeRef
```

### 1.2 의미론

인터페이스 선언은 구현 없는 메서드와 프로퍼티의 계약을 정의한다. `.prsm` 파일당 하나의 최상위 선언 규칙을 따른다.

```prsm
interface IDamageable {
    func takeDamage(amount: Int)
    val isAlive: Bool
}

interface IHealable : IDamageable {
    func heal(amount: Int)
}
```

### 1.3 Lowering

```csharp
public interface IDamageable
{
    void takeDamage(int amount);
    bool isAlive { get; }
}

public interface IHealable : IDamageable
{
    void heal(int amount);
}
```

### 1.4 구현

`component`, `class`에서 인터페이스를 구현한다. 컴파일러는 모든 인터페이스 멤버가 구현되었는지 검증해야 한다(shall).

```prsm
component Enemy : MonoBehaviour, IDamageable {
    var hp: Int = 100
    val isAlive: Bool = hp > 0

    func takeDamage(amount: Int) {
        hp -= amount
    }
}
```

### 1.5 진단

| 코드 | 조건 |
|------|------|
| E090 | 인터페이스 멤버가 구현되지 않음 |
| E091 | 인터페이스에 구현 본문이 포함됨 (금지) |

---

## 2. 제네릭 선언 [decl.generic]

### 2.1 문법

```ebnf
GenericParams = "<" GenericParam { "," GenericParam } ">"

GenericParam = Identifier

WhereClause = "where" Identifier ":" TypeRef { "," TypeRef }

ClassDecl = "class" Identifier [ GenericParams ] [ ":" SuperTypes ] [ WhereClause ]
            "{" { Member } "}"

FuncDecl = "func" Identifier [ GenericParams ] "(" [ ParamList ] ")" [ ":" TypeRef ]
           [ WhereClause ] ( Block | "=" Expr )
```

### 2.2 의미론

클래스와 함수에 타입 파라미터를 선언할 수 있다. `where` 절로 타입 제약을 명시한다.

```prsm
class Registry<T> where T : Component {
    var items: List<T> = null

    func register(item: T) {
        items.add(item)
    }

    func findFirst(): T? {
        return items.firstOrDefault()
    }
}
```

```prsm
func findAll<T>(): List<T> where T : Component {
    return FindObjectsByType<T>(FindObjectsSortMode.None).toList()
}
```

### 2.3 Lowering

PrSM 제네릭은 C# 제네릭으로 1:1 매핑된다.

```csharp
public class Registry<T> where T : Component
{
    public List<T> items;

    public void register(T item)
    {
        items.Add(item);
    }

    public T findFirst()
    {
        return items.FirstOrDefault();
    }
}
```

### 2.4 제약

- 타입 파라미터에 `where T : Type` 단일 또는 복수 제약 가능
- `where T : MonoBehaviour, IDamageable` — C#의 복합 제약과 동일
- `component`, `asset`, `enum`, `data class`에는 제네릭 파라미터를 선언할 수 없다(shall not). 오직 `class`와 `func`만 가능
- 제네릭 `interface`는 지원한다: `interface IPool<T> { func get(): T }`

### 2.5 진단

| 코드 | 조건 |
|------|------|
| E095 | 타입 제약 위반 (실 인자가 where 절을 만족하지 않음) |
| E096 | component/asset/enum/data class에 제네릭 파라미터 선언 |

---

## 3. C# 코드 옵티마이저 [opt]

### 3.1 개요

컴파일러 파이프라인에 선택적 최적화 패스를 추가한다. Lowering과 CodeGen 사이에 위치한다.

```
Parser → Semantic → HIR → Lowering → C# IR → [Optimizer] → CodeGen
```

`--optimize` 플래그로 활성화한다. 기본값은 비활성 (읽기 쉬운 C# 우선).

```bash
prism build --optimize
```

`.prsmproject`에서도 설정 가능:

```toml
[compiler]
optimize = true
```

### 3.2 최적화 규칙

각 규칙은 독립적으로 적용된다. 최적화는 프로그램의 관찰 가능한 동작을 변경해서는 안 된다(shall not).

#### 3.2.1 GetComponent 캐싱 [opt.cache]

동일 메서드 내에서 같은 타입의 `GetComponent<T>()` 호출이 2회 이상 발견되면, 메서드 시작에 로컬 변수로 캐싱한다.

Before:
```csharp
var rb = GetComponent<Rigidbody>();
rb.velocity = Vector3.zero;
// ... 이후
GetComponent<Rigidbody>().AddForce(force);
```

After:
```csharp
var _cached_rb = GetComponent<Rigidbody>();
_cached_rb.velocity = Vector3.zero;
// ...
_cached_rb.AddForce(force);
```

#### 3.2.2 임시 변수 제거 [opt.temp]

컴파일러가 생성한 임시 변수(`_prsm_d`, `_prsm_m` 등)가 단일 사용이면 인라인한다.

Before:
```csharp
var _prsm_d = GetStats();
var hp = _prsm_d.hp;
```

After:
```csharp
var hp = GetStats().hp;
```

단, 식에 부작용(side effect)이 있으면 인라인하지 않는다.

#### 3.2.3 널 체크 병합 [opt.null]

연속된 널 체크를 C# 널 조건 연산자로 병합한다.

Before:
```csharp
if (a != null)
{
    if (a.b != null)
    {
        a.b.c();
    }
}
```

After:
```csharp
a?.b?.c();
```

#### 3.2.4 박싱 회피 [opt.boxing]

값 타입 컬렉션의 `foreach`를 인덱스 기반 `for`로 변환한다. IL2CPP에서 박싱 할당을 줄인다.

Before:
```csharp
foreach (var item in intList)
{
    Process(item);
}
```

After:
```csharp
for (int _i = 0; _i < intList.Count; _i++)
{
    var item = intList[_i];
    Process(item);
}
```

#### 3.2.5 상수 문자열 접기 [opt.const]

문자열 보간에서 모든 부분이 상수이면 컴파일 타임에 연결한다.

Before: `$"hello {"world"}"` → After: `"hello world"`

#### 3.2.6 불필요한 캐스트 제거 [opt.cast]

`GetComponent<T>()`의 반환 타입이 이미 `T`인 경우 불필요한 캐스트를 제거한다.

---

## 4. `singleton` 키워드 [decl.singleton]

### 4.1 문법

```ebnf
SingletonDecl = "singleton" ComponentDecl
```

`singleton`은 `component` 앞에만 사용할 수 있다(shall).

### 4.2 의미론

`singleton component`는 다음을 자동 생성한다:

1. `private static T _instance` 필드
2. `public static T Instance` 프로퍼티 (지연 초기화 + FindFirstObjectByType 폴백)
3. `Awake()`에서 중복 인스턴스 파괴 + `DontDestroyOnLoad`
4. 사용자의 `awake` 블록은 자동 생성 코드 이후에 실행

### 4.3 예시

```prsm
singleton component AudioManager : MonoBehaviour {
    serialize volume: Float = 1.0

    func playSound(clip: AudioClip) {
        // ...
    }
}

// 다른 컴포넌트에서
AudioManager.instance.playSound(clip)
```

### 4.4 Lowering

```csharp
public class AudioManager : MonoBehaviour
{
    [SerializeField] private float _volume = 1.0f;

    private static AudioManager _instance;
    public static AudioManager Instance
    {
        get
        {
            if (_instance == null)
            {
                _instance = FindFirstObjectByType<AudioManager>();
                if (_instance == null)
                {
                    var go = new GameObject(nameof(AudioManager));
                    _instance = go.AddComponent<AudioManager>();
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
            return;
        }

        // 사용자 awake 블록 코드 ...
    }

    public void playSound(AudioClip clip)
    {
        // ...
    }
}
```

### 4.5 진단

| 코드 | 조건 |
|------|------|
| E097 | `singleton`이 `component` 이외의 선언에 사용됨 |
| W015 | 싱글톤 컴포넌트가 `require` 필드를 가짐 (의존성 주입과 싱글톤 혼용 주의) |

---

## 5. `pool` 수식자 [decl.pool]

### 5.1 문법

```ebnf
PoolField = "pool" Identifier ":" TypeRef "(" PoolArgs ")"

PoolArgs = PoolArg { "," PoolArg }

PoolArg = "capacity" "=" IntLiteral
        | "max" "=" IntLiteral
```

`pool`은 `component` 내부에서만 사용할 수 있다(shall).

### 5.2 의미론

`pool name: Type(capacity = N, max = M)`은 `UnityEngine.Pool.ObjectPool<T>`를 자동 생성한다.

- `name.get()` — 풀에서 오브젝트를 가져온다 (없으면 Instantiate)
- `name.release(obj)` — 오브젝트를 풀에 반환한다

### 5.3 예시

```prsm
component BulletSpawner : MonoBehaviour {
    serialize bulletPrefab: Bullet

    pool bullets: Bullet(capacity = 20, max = 100)

    func fire(direction: Vector3) {
        val bullet = bullets.get()
        bullet.transform.position = transform.position
        bullet.launch(direction)
    }
}
```

### 5.4 Lowering

```csharp
public class BulletSpawner : MonoBehaviour
{
    [SerializeField] private Bullet _bulletPrefab;

    private ObjectPool<Bullet> _bullets;

    private void Awake()
    {
        _bullets = new ObjectPool<Bullet>(
            createFunc: () => Instantiate(_bulletPrefab),
            actionOnGet: obj => obj.gameObject.SetActive(true),
            actionOnRelease: obj => obj.gameObject.SetActive(false),
            actionOnDestroy: obj => Destroy(obj.gameObject),
            collectionCheck: true,
            defaultCapacity: 20,
            maxSize: 100
        );
    }

    public void fire(Vector3 direction)
    {
        var bullet = _bullets.Get();
        bullet.transform.position = transform.position;
        bullet.launch(direction);
    }
}
```

### 5.5 프리팹 연결

풀의 타입 이름과 일치하는 `serialize` 필드가 존재하면, 해당 필드를 프리팹으로 사용한다. 없으면 진단 에러.

### 5.6 진단

| 코드 | 조건 |
|------|------|
| E098 | `pool` 타입과 일치하는 serialize 프리팹이 없음 |
| E099 | `pool`이 component 외부에서 사용됨 |

---

## 6. SOLID 분석 경고 [analysis.solid]

### 6.1 개요

시맨틱 분석에 정적 분석 패스를 추가한다. 모든 SOLID 경고는 프로젝트 설정에서 비활성화할 수 있다.

```toml
[analysis]
solid_warnings = true       # 전체 ON/OFF
disabled_warnings = []      # 개별 비활성화: ["W010", "W012"]
```

### 6.2 경고 코드

#### W010 — 과다 책임 (단일 책임 원칙)

```
W010: Component 'PlayerController' has 12 public methods.
      Consider splitting into smaller, focused components.
```

조건: component의 public 메서드(func + lifecycle) 수가 임계값(기본 8)을 초과.
임계값은 설정 가능:

```toml
[analysis]
max_public_methods = 10
```

#### W011 — 과다 의존성 (종속성 역전 원칙)

```
W011: Component 'GameManager' has 7 require/optional fields.
      Consider reducing dependencies or using an event-based architecture.
```

조건: require + optional + child + parent 필드 합계가 임계값(기본 6)을 초과.

#### W012 — 과도한 메서드 길이 (단일 책임 원칙)

```
W012: Method 'update' in 'PlayerController' has 65 statements.
      Consider extracting helper methods.
```

조건: 단일 메서드/라이프사이클 블록의 문(statement) 수가 임계값(기본 50)을 초과.

#### W013 — 구체 타입 의존 (종속성 역전 원칙)

```
W013: Field 'require weaponSystem: WeaponSystem' depends on a concrete type.
      Consider using an interface type for better decoupling.
```

조건: `require`/`optional` 필드의 타입이 인터페이스가 아닌 구체 component/class.
이 경고는 인터페이스 선언(§1) 기능과 연동된다 — 인터페이스를 선언할 수 있어야 의미가 있다.

---

## 7. Feature Gate 확장

Language 3 기능은 다음 feature flag로 제어된다:

```toml
[language]
version = "3"
features = ["interface", "generics", "optimizer", "singleton", "pool", "solid-analysis"]
```

`version = "3"`일 때 모든 Language 3 기능이 암묵적으로 활성화된다.

개별 feature flag로 선택적 활성화도 가능:

```toml
[language]
version = "2"
features = ["interface", "generics"]  # Language 2에서 일부 Language 3 기능만 사용
```

---

## 8. 기존 기능과의 상호작용

### 8.1 인터페이스 + require

`require` 필드에 인터페이스 타입을 사용할 수 있다:

```prsm
interface IDamageable {
    func takeDamage(amount: Int)
}

component Weapon : MonoBehaviour {
    require target: IDamageable  // GetComponent<IDamageable>()로 lowering
}
```

### 8.2 제네릭 + 타입 추론 (Language 2)

Language 2의 제네릭 추론은 Language 3의 사용자 정의 제네릭에도 적용된다:

```prsm
class Wrapper<T> where T : Component {
    func wrap(item: T): Wrapper<T> { /* ... */ }
}

val wrapped: Wrapper<Rigidbody> = Wrapper()  // T = Rigidbody 추론
```

### 8.3 singleton + listen

싱글톤 컴포넌트에서도 `listen` 수명 정책이 정상 동작한다:

```prsm
singleton component EventBus : MonoBehaviour {
    listen globalEvent until destroy {
        handleEvent()
    }
}
```

### 8.4 pool + 옵티마이저

옵티마이저가 풀의 `get()` 반환값에 대한 불필요한 임시 변수를 제거할 수 있다.

---

## 9. 에러 코드 요약

| 코드 | 심각도 | 기능 | 메시지 |
|------|--------|------|--------|
| E090 | Error | 인터페이스 | 인터페이스 멤버 '{name}' 미구현 |
| E091 | Error | 인터페이스 | 인터페이스에 구현 본문 포함 불가 |
| E095 | Error | 제네릭 | 타입 인자 '{T}'가 제약 '{constraint}'를 만족하지 않음 |
| E096 | Error | 제네릭 | component/asset/enum/data class에 제네릭 파라미터 선언 불가 |
| E097 | Error | 싱글톤 | 'singleton'은 component에만 사용 가능 |
| E098 | Error | 풀 | pool 타입 '{T}'에 대한 serialize 프리팹 필드가 없음 |
| E099 | Error | 풀 | 'pool'은 component 내부에서만 사용 가능 |
| W010 | Warning | SOLID | 컴포넌트의 public 메서드가 {N}개 (임계값 {M}) |
| W011 | Warning | SOLID | 컴포넌트의 의존성 필드가 {N}개 (임계값 {M}) |
| W012 | Warning | SOLID | 메서드 '{name}'의 문장이 {N}개 (임계값 {M}) |
| W013 | Warning | SOLID | require/optional 필드 '{name}'이 구체 타입에 의존 |
| W015 | Warning | 싱글톤 | 싱글톤 컴포넌트에 require 필드 존재 (설계 주의) |

---

## 10. 구현 순서

표준 문서 작성 → 컴파일러 구현 순서:

1. **인터페이스 선언** — 파서 + AST + 시맨틱 + lowering
2. **제네릭 선언** — 파서 + AST + 시맨틱 + lowering + 타입 추론 연동
3. **`singleton` 키워드** — 파서 + lowering (시맨틱은 기존 component 재사용)
4. **`pool` 수식자** — 파서 + lowering (프리팹 연결 검증)
5. **SOLID 분석** — 시맨틱 분석 패스 추가 (기존 코드 무수정)
6. **C# 코드 옵티마이저** — lowering 후 별도 패스 (기존 파이프라인 무수정)

---

## 11. 릴리스 조건

- 6개 기능 모두 구현 및 테스트 완료
- 각 기능에 대한 골든 테스트 존재
- `docs/en/spec/standard.md`가 Language 3으로 갱신
- `docs/en/spec/changes-lang-3.md` 작성
- `prism version`이 1.0.0 출력
- MSI/winget/Marketplace 릴리스 성공
