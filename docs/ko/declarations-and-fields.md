---
title: Declarations & Fields
parent: 언어 가이드
grand_parent: 한국어 문서
nav_order: 5
---

# Declarations & Fields

## 최상위 선언

`.prsm` 파일 하나에는 정확히 하나의 최상위 선언만 들어갑니다.

| 키워드 | C# 대응 | 목적 |
|---|---|---|
| `component` | `MonoBehaviour` 서브클래스 | GameObject에 부착하는 게임플레이 로직 |
| `asset` | `ScriptableObject` 서브클래스 | 데이터 컨테이너, 설정, 공유 상태 |
| `class` | 일반 C# `class` | 유틸리티, 서비스, 순수 데이터 |
| `data class` | 직렬화 가능한 값 클래스 | 생성 equality를 갖는 경량 데이터 |
| `enum` | `enum` | 명명된 상수 집합 |
| `attribute` | `Attribute` 서브클래스 | 커스텀 C# 어노테이션 |

## `component`

```prsm
using UnityEngine

component PlayerController : MonoBehaviour {
    @header("이동")
    serialize speed: Float = 5.0

    require rb: Rigidbody

    update {
        move()
    }

    func move() {
        rb.MovePosition(rb.position + transform.forward * speed * Time.fixedDeltaTime)
    }
}
```

## `asset`

```prsm
using UnityEngine

asset WeaponConfig : ScriptableObject {
    serialize damage: Int = 10
    serialize fireRate: Float = 0.2
    serialize projectilePrefab: GameObject = null
}
```

Unity 에디터에서 `ScriptableObject.CreateInstance<T>()`로 생성한 에셋은 값을 `.asset` 파일에 영구 저장합니다.

## `class`

```prsm
class DamageCalculator {
    func calculate(base: Int, multiplier: Float): Float {
        return base * multiplier
    }
}
```

`class`는 Unity 의존성 없이 일반 C# 클래스로 매핑됩니다.

## 직렬화 필드

`serialize`로 표시된 필드는 Unity Inspector에 노출됩니다. 데코레이터 어노테이션으로 표시 방식을 제어합니다.

```prsm
@header("스탯")
serialize maxHp: Int = 100

@tooltip("초당 이동 거리")
serialize speed: Float = 5.0

@range(0.0, 1.0)
serialize damageMultiplier: Float = 0.5

@space
serialize weaponSlot: GameObject = null
```

지원 데코레이터: `@header(label)`, `@tooltip(text)`, `@range(min, max)`, `@space`, `@hideInInspector`.

Input System asset wiring 용도로는, 직렬화된 `InputActionAsset` 필드에
compiler 전용 annotation `@inputActions(defaultMap: "Gameplay")`를 붙일 수
있습니다. 이 annotation은 C# attribute를 내보내는 대신 생성된
`PlayerInput` wiring 을 구성합니다.

## `val`과 `var`

- `val` — 초기화 후 재할당 불가
- `var` — 가변 필드 또는 로컬

```prsm
val gravity: Float = 9.81      // 상수
var hp: Int = 100               // 가변
```

## 가시성 한정자

`public`, `private`, `protected`는 C#에 그대로 매핑됩니다. 대부분 컨텍스트에서 기본값은 `public`입니다.

```prsm
private var invincible: Bool = false
protected var baseSpeed: Float = 5.0
```

## 컴포넌트 룩업 필드

아래 네 가지 한정자는 `component` 선언 안에서만 유효합니다. 생성된 `Awake()` 안에서 사용자 `awake` 바디보다 **먼저** 룩업 코드를 실행합니다.

| 한정자 | 생성되는 C# | null 계약 |
|---|---|---|
| `require name: Type` | `GetComponent<Type>()` | 없으면 오류 로그 + 비null 보장 |
| `optional name: Type?` | `GetComponent<Type>()` | null 허용, nullable로 저장 |
| `child name: Type` | `GetComponentInChildren<Type>()` | 비null 보장 |
| `parent name: Type` | `GetComponentInParent<Type>()` | 비null 보장 |

```prsm
require animator: Animator
optional shield: Shield?
child muzzle: Transform
parent vehicle: Vehicle
```

이 한정자들은 component에서만 유효합니다 (class/asset에서 사용 시 에러 E013).

## `data class`

Data class는 생성자, `Equals`, `GetHashCode`, `ToString`이 포함된 C# 클래스를 생성합니다:

```prsm
data class DamageInfo(amount: Int, crit: Bool)
```

생성 C#:

```csharp
[System.Serializable]
public class DamageInfo {
    public int amount;
    public bool crit;

    public DamageInfo(int amount, bool crit) { ... }
    public override bool Equals(object obj) { ... }
    public override int GetHashCode() { ... }
    public override string ToString() {
        return $"DamageInfo(amount={amount}, crit={crit})";
    }
}
```

Data class는 v2 구조 분해를 지원합니다: `val DamageInfo(amount, crit) = info`.

## `enum` (파라미터화)

단순 enum은 C# enum으로 직접 매핑됩니다:

```prsm
enum Direction { Up, Down, Left, Right }
```

파라미터화 enum은 enum + payload 접근 확장 메서드를 생성합니다:

```prsm
enum Weapon(val damage: Int, val range: Float) {
    Sword(10, 1.5),
    Bow(7, 8.0)
}
```

**규칙:**
- 모든 엔트리는 enum 파라미터와 같은 수의 인자를 제공해야 합니다 (에러 E051)
- 최소 하나의 엔트리가 필요합니다 (에러 E050)
- 중복 엔트리 이름 불가 (에러 E052)

## `attribute`

직렬화 필드용 커스텀 어트리뷰트:

```prsm
attribute Cooldown(val duration: Float, val label: String)
```

필드 데코레이터로 사용: `@cooldown(2.0, "Fire Rate")`.

## `interface` (PrSM 3 부터)

Interface는 구현 없이 메서드 시그니처와 프로퍼티를 정의합니다:

```prsm
interface IDamageable {
    func takeDamage(amount: Int)
    val isAlive: Bool
}

interface IHealable : IDamageable {
    func heal(amount: Int)
}
```

생성 C#:

```csharp
public interface IDamageable
{
    void takeDamage(int amount);
    bool isAlive { get; }
}
```

Component와 class는 기본 클래스 뒤에 interface를 구현합니다:

```prsm
component Enemy : MonoBehaviour, IDamageable {
    var hp: Int = 100
    val isAlive: Bool = hp > 0
    func takeDamage(amount: Int) { hp -= amount }
}
```

Interface는 `require`와 함께 사용할 수 있습니다: `require target: IDamageable`.

구현되지 않은 멤버는 에러 E090을, interface 안의 구현 본문은 E091을 발생시킵니다.

## `singleton` component (PrSM 3 부터)

component에 `singleton` 수정자를 붙이면 싱글톤 패턴이 자동으로 생성됩니다:

```prsm
singleton component AudioManager : MonoBehaviour {
    serialize volume: Float = 1.0
    func playSound(clip: AudioClip) { /* ... */ }
}
```

정적 `Instance` 프로퍼티(지연 초기화), `DontDestroyOnLoad`가 포함된 `Awake` 중복 체크가 생성됩니다. `AudioManager.instance.playSound(clip)`으로 접근합니다.

`singleton`은 `component` 선언에서만 유효합니다 (E097).

## `struct` 선언 (PrSM 4 부터)

`struct`는 복사 의미와 스택 할당을 갖는 값 타입을 선언합니다. `data class`와 유사하지만 C# `struct`로 저장됩니다.

```prsm
struct DamageInfo(amount: Int, type: DamageType)

struct Color32(r: Byte, g: Byte, b: Byte, a: Byte) {
    static val white = Color32(255, 255, 255, 255)
    static val black = Color32(0, 0, 0, 255)
}
```

생성 C#:

```csharp
public struct DamageInfo {
    public int amount;
    public DamageType type;
    public DamageInfo(int amount, DamageType type) { this.amount = amount; this.type = type; }
}
```

`struct`는 클래스를 상속할 수 없습니다 (E120). 16바이트를 초과하는 구조체는 잠재적 복사 비용을 표시하기 위해 W022를 발생시킵니다.

## 프로퍼티 `get` / `set` (PrSM 4 부터)

프로퍼티는 사용자 정의 접근자를 가질 수 있습니다. `get`만 있는 `val`은 계산 전용 읽기 전용 프로퍼티입니다. `set` 내부에서 암묵적 `field` 키워드는 자동 생성된 백킹 필드를 가리킵니다 (Kotlin 스타일).

```prsm
component Player : MonoBehaviour {
    var hp: Int
        get = _hp
        set(value) {
            _hp = Mathf.clamp(value, 0, maxHp)
            onHpChanged?.invoke(_hp)
        }

    val isAlive: Bool
        get = hp > 0
}
```

생성 C#:

```csharp
private int __hp;
public int hp {
    get => __hp;
    set {
        __hp = Mathf.Clamp(value, 0, maxHp);
        onHpChanged?.Invoke(__hp);
    }
}
public bool isAlive => hp > 0;
```

`val`에 `set`이 선언되면 E110이 발생합니다. 프로퍼티 접근자 외부에 `field`가 나타나면 E111이 발생합니다.

## `event` 멤버 (PrSM 4 부터)

`event`는 `+=` / `-=` 구독과 `.invoke()` 디스패치를 지원하는 멀티캐스트 델리게이트를 선언합니다:

```prsm
component Damageable : MonoBehaviour {
    event onHealthChanged: (Int) => Unit

    func takeDamage(amount: Int) {
        hp -= amount
        onHealthChanged.invoke(hp)
    }
}

// 다른 곳에서:
target.onHealthChanged += { hp => log("HP: $hp") }
```

생성 C#:

```csharp
public event Action<int> onHealthChanged;
// ...
onHealthChanged?.Invoke(hp);
```

이벤트는 선언 타입 내부에서만 호출할 수 있고 (E121) `Unit` 반환 타입을 가져야 합니다 (E122). 함수 타입 콜백 (`var onDamaged: ((Int) => Unit)? = null`)은 `event` 키워드 없이 단일 캐스트 등가물을 제공합니다.

## 확장 메서드 — `extend` (PrSM 4 부터)

`extend Type { }`은 기존 타입에 메서드와 계산 프로퍼티를 추가합니다. 블록 내부에서 `this`는 수신자 인스턴스를 가리킵니다:

```prsm
extend Transform {
    func resetLocal() {
        this.localPosition = Vector3.zero
        this.localRotation = Quaternion.identity
        this.localScale = Vector3.one
    }
}

extend Vector3 {
    val flat: Vector3
        get = vec3(this.x, 0, this.z)
}

transform.resetLocal()
val groundPos = transform.position.flat
```

C# 확장 메서드를 가진 정적 클래스로 변환됩니다. 확장 블록은 가변 상태를 포함할 수 없습니다 (E113).

## `bind` 반응형 프로퍼티 (PrSM 4 부터)

`bind`는 값이 변경될 때 자동으로 구독자에게 알리는 반응형 프로퍼티를 선언합니다. 컴파일러가 `INotifyPropertyChanged` 배관을 생성합니다:

```prsm
component PlayerHUD : MonoBehaviour {
    bind hp: Int = 100
    bind playerName: String = "Hero"

    serialize hpLabel: TextMeshProUGUI

    awake {
        bind hp to hpLabel.text
    }
}
```

생성 C#:

```csharp
private int _hp = 100;
public int hp {
    get => _hp;
    set {
        if (_hp != value) {
            _hp = value;
            OnPropertyChanged(nameof(hp));
        }
    }
}

public event PropertyChangedEventHandler PropertyChanged;
```

`bind to` 타깃이 쓰기 가능하지 않으면 E143이 발생합니다. 소스와 타깃 사이 타입 불일치 시 E144가 발생합니다. W031은 읽히지 않는 bind 프로퍼티를 경고합니다.

## `command` 선언 (PrSM 4 부터)

`command`는 선택적 undo 지원과 가드 조건을 가진 실행 가능 액션을 선언합니다. 컴파일러가 중첩된 `ICommand` 구현을 생성합니다:

```prsm
component UnitController : MonoBehaviour {
    command moveUnit(target: Vector3) {
        val prevPos = transform.position
        transform.position = target
    } undo {
        transform.position = prevPos
    } canExecute = isAlive && !isStunned
}
```

컴파일러는 `Execute()`, `Undo()`, `CanExecute()`를 가진 `MoveUnitCommand : ICommand` 클래스와 소유자의 헬퍼 메서드를 생성합니다. `canExecute` 가드 없는 command는 W030을 발생시킵니다.

## `state machine` 선언 (PrSM 4 부터)

`state machine`은 명명된 상태, 전이, 선택적 `enter`/`exit` 훅을 가진 유한 상태 머신을 선언합니다:

```prsm
component EnemyAI : MonoBehaviour {
    state machine aiState {
        state Idle {
            enter { playAnimation("idle") }
            on playerDetected => Chase
        }
        state Chase {
            enter { playAnimation("run") }
            on playerLost => Idle
            on inAttackRange => Attack
        }
        state Attack {
            enter { playAnimation("attack") }
            exit  { resetCooldown() }
            on attackDone => Chase
        }
    }
}
```

컴파일러는 private 상태 enum, 현재 상태 필드, public `TransitionAiState(string event)` 디스패처, `_enterAiState`/`_exitAiState` 헬퍼를 생성합니다. 선언되지 않은 상태로의 전이는 E140을, 중복 상태 이름은 E141을, 나가는 전이가 없는 상태는 W029를 발생시킵니다.

## `pool` 수정자 (PrSM 3 부터)

`pool` 수정자는 `UnityEngine.Pool.ObjectPool<T>`를 기반으로 오브젝트 풀을 생성합니다:

```prsm
component BulletSpawner : MonoBehaviour {
    serialize bulletPrefab: Bullet
    pool bullets: Bullet(capacity = 20, max = 100)

    func fire(direction: Vector3) {
        val bullet = bullets.get()
        bullet.launch(direction)
    }
}
```

풀은 프리팹을 위해 동일한 타입의 `serialize` 필드를 자동으로 매칭합니다. 프리팹이 없으면 E098이, `pool`이 component 외부에 있으면 E099가 발생합니다.

## 프로퍼티의 어트리뷰트 타깃 (PrSM 5 부터)

자동 프로퍼티에 `serialize` 한정자가 붙으면 `[field: SerializeField]` 백킹 필드로 변환됩니다 — 자동 프로퍼티를 인스펙터에 노출하면서 공개 표면은 프로퍼티로 유지하는 Unity 표준 패턴입니다. 일반 형식인 `@field(name)`, `@property(name)`, `@param(name)`, `@return(name)`, `@type(name)`을 사용하면 임의 선언의 비기본 타깃에 임의의 C# 어트리뷰트를 부착할 수 있습니다.

```prsm
component Player : MonoBehaviour {
    serialize var hp: Int = 100
        get
        set { field = Mathf.clamp(value, 0, maxHp) }

    @field(nonSerialized)
    var transientCache: Map<String, Int>

    @return(notNull)
    func getTarget(): Transform = currentTarget
}
```

```csharp
[field: SerializeField]
public int hp
{
    get;
    set { field = Mathf.Clamp(value, 0, maxHp); }
}

[field: NonSerialized]
public Dictionary<string, int> transientCache { get; set; }

[return: NotNull]
public Transform getTarget() => currentTarget;
```

자동 프로퍼티의 `serialize`는 `@field(serializeField)`의 편의 표기입니다. 선택한 선언이 어트리뷰트 타깃을 지원하지 않으면 E149, 자동 프로퍼티로 변환할 수 없는 액세서를 가진 프로퍼티에 `serialize`가 붙으면 E150이 발생합니다.

## `partial` 선언 (PrSM 5 부터)

`partial` 선언은 특정 타입에 대해 "파일당 단일 선언" 규칙을 완화합니다. 모든 부분이 동일한 식별자, `partial` 한정자, 동일한 타입 매개변수 / where clause를 공유하면 여러 파일이 같은 `partial` 선언에 기여할 수 있습니다.

`Player.prsm`:
```prsm
partial component Player : MonoBehaviour {
    serialize speed: Float = 5.0
    require rb: Rigidbody

    update { move() }
}
```

`Player.combat.prsm`:
```prsm
partial component Player {
    bind hp: Int = 100

    func takeDamage(amount: Int) {
        hp -= amount
        if hp <= 0 { die() }
    }
}
```

컴파일러는 lowering 중 모든 부분을 결합하여 단일 C# `partial class` (또는 `partial struct`)를 출력합니다. `partial`은 컨텍스트 키워드입니다. 같은 이름의 두 선언 중 하나만 `partial`을 가지면 E184, 부분 간 타입 매개변수 불일치는 E185, 베이스 클래스 또는 인터페이스 불일치는 E186이 발생합니다.

## 일반화된 nested 선언 (PrSM 5 부터)

모든 `class`, `component`, `struct` 본문에 nested 타입 선언을 둘 수 있습니다. 언어 4는 `sealed class` 안에서만 nested 선언을 허용했고, 언어 5는 이를 일반화합니다.

```prsm
component Inventory : MonoBehaviour {
    data class Slot(item: Item, count: Int)

    enum SortOrder { ByName, ByValue, ByRarity }

    var slots: List<Slot> = []
    var sortOrder: SortOrder = SortOrder.ByName

    func addItem(item: Item) {
        slots.add(Slot(item, 1))
    }
}
```

```csharp
public class Inventory : MonoBehaviour
{
    [System.Serializable]
    public class Slot
    {
        public Item item;
        public int count;
        public Slot(Item item, int count) { this.item = item; this.count = count; }
    }

    public enum SortOrder { ByName, ByValue, ByRarity }
    public List<Slot> slots = new List<Slot>();
    public SortOrder sortOrder = SortOrder.ByName;
    public void addItem(Item item) { slots.Add(new Slot(item, 1)); }
}
```

nested `component` 선언은 금지됩니다 (E187). 컴포넌트는 top-level이어야 합니다.

## `ref struct` 선언 (PrSM 5 부터)

`ref struct`는 `ref` 필드를 포함할 수 있는 스택 전용 값 타입을 선언합니다. C# ref struct 제약을 따릅니다: 비-ref struct의 필드가 될 수 없고, 박싱될 수 없으며, C# 13+의 `allows ref struct` 없이는 제네릭 타입 인자로 사용할 수 없습니다.

```prsm
ref struct Slice<T>(start: Int, length: Int) {
    func get(i: Int): T = intrinsic { return _data[start + i]; }
}
```

```csharp
public ref struct Slice<T>
{
    public int start;
    public int length;
    public Slice(int start, int length) { this.start = start; this.length = length; }
    public T get(int i) { return _data[start + i]; }
}
```

비-ref struct 또는 클래스의 필드로 선언된 `ref struct`는 E179, `allows ref struct` 없이 제네릭 타입 인자로 사용된 `ref struct`는 E180을 발생시킵니다.

## `@burst` 어노테이션 (PrSM 5 부터)

`@burst`는 함수나 struct를 Unity Burst 컴파일 대상으로 표시합니다. 컴파일러는 대응하는 `[BurstCompile]` 어트리뷰트를 출력하고, 언어 4의 Burst 호환성 분석기를 어노테이션 대상에 실행합니다. 어노테이션 옵션은 일치하는 어트리뷰트 인자로 변환됩니다.

```prsm
@burst
func calculateForces(positions: NativeArray<Float3>, forces: NativeArray<Float3>) {
    for i in 0..positions.length {
        forces[i] = computeGravity(positions[i])
    }
}

@burst(compileSynchronously = true)
struct DamageJob : IJobParallelFor {
    var damages: NativeArray<Int>
    func execute(index: Int) {
        damages[index] = damages[index] * 2
    }
}
```

언어 4의 진단 E137–E139와 W028은 이제 명명 휴리스틱(`burst_*`) 대신 `@burst` 어노테이션을 통해 발생합니다. `component`, `asset`, `interface`처럼 지원되지 않는 선언 종류에 `@burst`가 붙으면 E165가 발생합니다.

## 초기화 순서

component의 초기화 순서:

1. Unity가 `Awake()` 호출
2. 컴파일러 생성: `require`/`optional`/`child`/`parent` 룩업 실행
3. 컴파일러 생성: 직렬화 필드 기본값 적용
4. 사용자 `awake { }` 바디 실행
5. Unity가 `Start()` 호출 → 사용자 `start { }` 바디 실행
