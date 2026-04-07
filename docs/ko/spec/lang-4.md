---
title: PrSM 4
parent: 사양
nav_order: 6
---

# PrSM 언어 4

PrSM 4는 지금까지 가장 큰 단일 언어 릴리스입니다. 문법 확장, 비동기 + 성능, 패턴 편의 기능, 개발자 경험의 네 가지 카테고리에 걸쳐 30개 기능을 추가하며 **Prism v2.0.0**으로 출시됩니다. 모든 언어 3 프로그램은 변경 없이 그대로 컴파일됩니다.

**활성화:** `.prsmproject`에서 `language.version = "4"` 설정

## Part I — 문법 확장

### `try` / `catch` / `finally` 와 `throw`

예외가 일급 시민이 됩니다. `throw`에서 `new` 키워드는 생략됩니다.

```prsm
try {
    val data = File.readAllText(path)
} catch (e: FileNotFoundException) {
    warn(e.message)
} catch (e: Exception) {
    error(e.message)
} finally {
    cleanup()
}

throw ArgumentException("Invalid value")

val result = try { parseInt(str) } catch (e: Exception) { -1 }
```

```csharp
try
{
    var data = File.ReadAllText(path);
}
catch (FileNotFoundException e) { Debug.LogWarning(e.Message); }
catch (Exception e) { Debug.LogError(e.Message); }
finally { Cleanup(); }

throw new ArgumentException("Invalid value");
```

`try`는 정확히 하나의 `catch` 절이 있을 때 표현식으로 사용할 수 있습니다.

### 람다식과 함수 타입

람다는 `{ }`로 둘러싸인 익명 함수입니다. 단일 매개변수 람다는 암묵적 `it` 식별자를 사용할 수 있습니다. 호출의 마지막 인자가 람다이면 괄호 밖으로 빼낼 수 있습니다(후행 람다).

```prsm
val callback: (Int) => Unit = { x => log(x) }
val add: (Int, Int) => Int = { a, b => a + b }

list.filter { it > 10 }
list.where({ x => x > 10 }).select({ x => x * 2 })

func doWork(callback: (Int) => Unit) { }
```

```csharp
Action<int> callback = (x) => Debug.Log(x);
Func<int, int, int> add = (a, b) => a + b;

list.Where(x => x > 10).Select(x => x * 2);
```

`(A, B) => R`은 `Func<A, B, R>`로, `() => Unit`은 `Action`으로 변환됩니다.

### `static` 과 `const`

`static`은 인스턴스 없이 접근 가능한 클래스 수준 멤버를 선언합니다. `const`는 리터럴 초기화자만 허용되는 컴파일 타임 상수를 선언합니다.

```prsm
class MathHelper {
    static val PI: Float = 3.14159
    static func lerp(a: Float, b: Float, t: Float): Float = a + (b - a) * t
}

const MAX_HEALTH: Int = 100
const VERSION: String = "1.0.0"

component ScoreManager : MonoBehaviour {
    static var highScore: Int = 0
}
```

```csharp
public class MathHelper
{
    public static readonly float PI = 3.14159f;
    public static float lerp(float a, float b, float t) => a + (b - a) * t;
}

public const int MAX_HEALTH = 100;
public const string VERSION = "1.0.0";
```

### 컬렉션 리터럴

`List<T>`, `Map<K, V>`, `Array<T>`, `Set<T>`을 위한 인라인 문법.

```prsm
val numbers = [1, 2, 3, 4, 5]
val names = ["Alice", "Bob", "Charlie"]
val empty: List<Int> = []

val lookup = {"hp": 100, "mp": 50}
val arr: Array<Int> = arrayOf(1, 2, 3)
val unique = setOf("Alice", "Bob", "Alice")
```

```csharp
var numbers = new List<int> { 1, 2, 3, 4, 5 };
var lookup = new Dictionary<string, int> { ["hp"] = 100, ["mp"] = 50 };
var arr = new int[] { 1, 2, 3 };
var unique = new HashSet<string> { "Alice", "Bob", "Alice" };
```

빈 컬렉션 리터럴은 명시적 타입 주석이 필요합니다 (E107).

### 타입 캐스팅

`as Type?`은 실패 시 `null`을 반환하는 안전 캐스트를 수행합니다. `as! Type`은 실패 시 예외를 던지는 강제 캐스트입니다. `is` 검사 후에는 같은 스코프 내에서 변수가 검사된 타입으로 스마트 캐스트됩니다.

```prsm
val enemy = collider as Enemy?
val boss = collider as! Boss

if collider is BoxCollider {
    log(collider.size)   // BoxCollider로 스마트 캐스트
}

when target {
    is Enemy => target.takeDamage(10)
    is Ally  => target.heal(5)
}

val f = 42.toFloat()
val s = 100.toString()
```

```csharp
var enemy = collider as Enemy;
var boss = (Boss)collider;

if (collider is BoxCollider _bc) { Debug.Log(_bc.size); }

switch (target)
{
    case Enemy _e: _e.takeDamage(10); break;
    case Ally _a: _a.heal(5); break;
}
```

### 프로퍼티 `get` / `set`

프로퍼티는 사용자 정의 접근자를 가질 수 있습니다. `get`만 있는 `val`은 계산 전용 읽기 전용 프로퍼티입니다. `set` 내부에서 암묵적 `field` 키워드는 자동 생성된 백킹 필드를 가리킵니다.

```prsm
var hp: Int
    get = _hp
    set(value) {
        _hp = Mathf.clamp(value, 0, maxHp)
        onHpChanged?.invoke(_hp)
    }

val isAlive: Bool
    get = hp > 0

var name: String = "Default"
    set(value) {
        field = value.trim()
        log("Name changed to $field")
    }
```

```csharp
private int __hp;
public int hp
{
    get => __hp;
    set
    {
        __hp = Mathf.Clamp(value, 0, maxHp);
        onHpChanged?.Invoke(__hp);
    }
}

public bool isAlive => hp > 0;
```

### 확장 메서드 (`extend`)

`extend Type { }`은 기존 타입에 메서드와 계산 프로퍼티를 추가합니다. 원본 소스를 수정하지 않습니다.

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

### `abstract` / `sealed` / `open`

기본적으로 클래스와 메서드는 final입니다. `open`은 상속과 오버라이드를 허용합니다. `abstract`는 서브클래스 구현을 요구합니다. `sealed`는 서브클래스를 같은 파일로 제한하여 `when` 검사의 완전성을 활성화합니다.

```prsm
abstract class Weapon {
    abstract func attack()
    open func reload() { }
}

class Sword : Weapon {
    override func attack() { swing() }
}

sealed class Shape {
    class Circle(radius: Float) : Shape
    class Rect(width: Float, height: Float) : Shape
    class Triangle(base: Float, height: Float) : Shape
}

when shape {
    is Shape.Circle   => PI * shape.radius * shape.radius
    is Shape.Rect     => shape.width * shape.height
    is Shape.Triangle => shape.base * shape.height / 2
}
```

### 튜플

튜플은 여러 값을 단일 합성 값으로 묶습니다. 명명된 튜플은 라벨로 접근할 수 있습니다. 튜플은 구조 분해를 지원합니다.

```prsm
func getResult(): (Int, String) = (42, "answer")
val (num, name) = getResult()

func getStats(): (hp: Int, mp: Int) = (hp: 100, mp: 50)
val stats = getStats()
log(stats.hp)
```

```csharp
public (int, string) getResult() => (42, "answer");
var (num, name) = getResult();

public (int hp, int mp) getStats() => (hp: 100, mp: 50);
var stats = getStats();
Debug.Log(stats.hp);
```

### `IDisposable`을 위한 `use`

`use`는 `IDisposable` 리소스의 자동 해제를 보장합니다. 블록 형식은 블록 종료 시점에, 선언 형식은 둘러싼 스코프 종료 시점에 해제합니다.

```prsm
use stream = FileStream(path, FileMode.Open) {
    val data = stream.readToEnd()
}

use val conn = DbConnection(connString)
// 스코프 종료 시 conn 자동 해제
```

```csharp
using (var stream = new FileStream(path, FileMode.Open))
{
    var data = stream.ReadToEnd();
}

using var conn = new DbConnection(connString);
```

### `struct` 선언

`struct`는 복사 의미와 스택 할당을 갖는 값 타입을 선언합니다.

```prsm
struct DamageInfo(amount: Int, type: DamageType)

struct Color32(r: Byte, g: Byte, b: Byte, a: Byte) {
    static val white = Color32(255, 255, 255, 255)
    static val black = Color32(0, 0, 0, 255)
}
```

```csharp
public struct DamageInfo
{
    public int amount;
    public DamageType type;
    public DamageInfo(int amount, DamageType type) { this.amount = amount; this.type = type; }
}
```

`struct`는 클래스를 상속할 수 없습니다 (E120). 16바이트를 초과하는 큰 구조체는 W022를 발생시킵니다.

### `event` (멀티캐스트 델리게이트)

함수 타입 `(A, B) => Unit`은 단일 캐스트 콜백으로 직접 사용할 수 있습니다. `event` 키워드는 `+=` / `-=` 와 `.invoke()`를 지원하는 멀티캐스트 델리게이트를 선언합니다.

```prsm
// 단일 캐스트 콜백
var onDamaged: ((Int, DamageType) => Unit)? = null
onDamaged?.invoke(amount, type)

// 멀티캐스트 이벤트
event onHealthChanged: (Int) => Unit

onHealthChanged += { hp => log("HP: $hp") }
onHealthChanged.invoke(currentHp)
```

```csharp
public Action<int, DamageType> onDamaged;
onDamaged?.Invoke(amount, type);

public event Action<int> onHealthChanged;
onHealthChanged += (hp) => Debug.Log($"HP: {hp}");
onHealthChanged?.Invoke(currentHp);
```

### 연산자 오버로딩

연산자 함수는 기호 연산자를 명명된 메서드에 매핑합니다. Kotlin 규칙을 따릅니다: `plus`, `minus`, `times`, `div`, `mod`, `compareTo`, `equals`, `unaryMinus`, `not`.

```prsm
data class Vec2i(x: Int, y: Int) {
    operator plus(other: Vec2i): Vec2i = Vec2i(x + other.x, y + other.y)
    operator minus(other: Vec2i): Vec2i = Vec2i(x - other.x, y - other.y)
    operator times(scalar: Int): Vec2i = Vec2i(x * scalar, y * scalar)
}

val c = Vec2i(1, 2) + Vec2i(3, 4)  // Vec2i(4, 6)
```

```csharp
public static Vec2i operator +(Vec2i self, Vec2i other)
    => new Vec2i(self.x + other.x, self.y + other.y);
```

### 인덱서

`operator get`과 `operator set`은 `[]` 문법을 사용한 인덱서 접근을 정의합니다.

```prsm
class Inventory {
    var items: List<Item> = []

    operator get(index: Int): Item = items[index]
    operator set(index: Int, value: Item) { items[index] = value }
}

val sword = inventory[0]
inventory[1] = shield
```

```csharp
public Item this[int index]
{
    get => items[index];
    set => items[index] = value;
}
```

### `typealias`

`typealias`는 기존 타입에 대한 컴파일 타임 별칭을 도입합니다. 별칭은 변환 단계에서 제거됩니다.

```prsm
typealias Position = Vector3
typealias EnemyList = List<Enemy>

val pos: Position = vec3(1, 2, 3)
val enemies: EnemyList = []
```

### 원시 문자열 리터럴

삼중 따옴표 문자열은 이스케이프 없이 줄바꿈과 특수 문자를 보존합니다. 보간은 여전히 활성화됩니다.

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

지원되는 곳에서는 C# 11 원시 문자열 리터럴로, 이전 타깃에서는 `@"..."` 축자 문자열로 변환됩니다.

### `in` 연산자

`in`은 범위, 리스트, 맵에 대한 멤버십을 테스트합니다.

```prsm
if x in 1..10 { log("In range") }
if name in ["Alice", "Bob"] { log("Known user") }
if key in lookup { log("Key exists") }
```

```csharp
if (x >= 1 && x <= 10) { Debug.Log("In range"); }
if (new List<string>{"Alice", "Bob"}.Contains(name)) { Debug.Log("Known user"); }
if (lookup.ContainsKey(key)) { Debug.Log("Key exists"); }
```

### OR 패턴

`when` 분기에서 쉼표로 구분된 여러 패턴은 개별 패턴 중 하나라도 매치되면 매치됩니다. OR 그룹의 모든 분기는 같은 변수 집합을 바인딩해야 합니다 (또는 어느 것도 바인딩하지 않아야 합니다).

```prsm
when direction {
    Direction.Up, Direction.Down    => handleVertical()
    Direction.Left, Direction.Right => handleHorizontal()
}
```

### 범위 패턴

`when` 분기 안의 `in low..high`는 닫힌 범위 `[low, high]` 내의 값을 매치합니다.

```prsm
when score {
    in 90..100 => "A"
    in 80..89  => "B"
    in 70..79  => "C"
    else       => "F"
}
```

### Null 병합 대입 (`?:=`)

`a ?:= b`는 `a`가 현재 `null`인 경우에만 `b`를 `a`에 대입합니다. 좌변은 nullable 가변 변수여야 합니다.

```prsm
var _instance: GameManager? = null

func getInstance(): GameManager {
    _instance ?:= FindFirstObjectByType<GameManager>()
    return _instance!!
}
```

```csharp
_instance ??= FindFirstObjectByType<GameManager>();
```

### 디폴트 인터페이스 메서드

인터페이스 메서드는 디폴트 본문을 포함할 수 있습니다. 구현체는 메서드를 생략하여 디폴트를 사용할 수 있습니다. C# 8 이상의 디폴트 인터페이스 메서드가 필요합니다.

```prsm
interface IMovable {
    val speed: Float
    func move(direction: Vector3) {
        transform.translate(direction * speed * Time.deltaTime)
    }
}

component Player : MonoBehaviour, IMovable {
    val speed: Float = 5.0
    // move()는 디폴트 구현 사용
}
```

## Part II — 비동기와 성능

### `async` / `await`

`async func`는 비동기 함수를 선언합니다. `await`는 awaited 태스크가 완료될 때까지 실행을 일시 중단합니다. Unity 컨텍스트에서는 컴파일러가 UniTask를 우선하며, 사용 불가 시 `Task`로 폴백합니다.

```prsm
async func loadData(url: String): String {
    val response = await Http.get(url)
    return response.body
}

async func initialize() {
    val config = await loadData("/api/config")
    val assets = await loadData("/api/assets")
    setup(config, assets)
}
```

```csharp
public async UniTask<string> loadData(string url)
{
    var response = await Http.Get(url);
    return response.body;
}
```

### 옵티마이저 강화

세 개의 추가 옵티마이저 패스가 Unity 특화 성능 패턴을 대상으로 합니다. 언어 3에서 도입된 동일한 `--optimize` 플래그로 활성화됩니다.

- **opt.string** — `update`/`fixedUpdate`/`lateUpdate`에서 반복되는 `$"..."` 표현식이 변경 감지 가드와 함께 백킹 필드로 캐시됩니다. 할당이 프레임당에서 변경당으로 이동합니다.
- **opt.linq** — 핫 패스 내부의 `List<T>`에 대한 단순 LINQ 체인 (`.Where(...).ToList()`, `.Select(...).ToList()`)이 enumerator 할당을 제거하기 위해 수동 `for` 루프로 재작성됩니다.
- **opt.structcopy** — 핫 패스의 큰 `Vector3`/`Quaternion`/`Matrix4x4` 로컬은 `ref readonly` 치환을 위해 표시됩니다.

옵티마이저는 영향받은 사이트에 대해 W026 (문자열 할당 캐시됨) 및 W027 (LINQ 재작성됨)을 발생시켜 개발자가 각 재작성을 감사할 수 있도록 합니다.

### Burst 호환성 분석

정적 분석이 메서드나 구조체가 Burst로 컴파일될 수 있는지 검사합니다. 보고:

- **E137** — Burst 대상 메서드 내 관리 타입 참조 (class, string, delegate)
- **E138** — Burst 대상 메서드 내 `try`/`catch` 또는 `throw`
- **E139** — Burst 컨텍스트에서 가상 또는 인터페이스 디스패치
- **W028** — Burst 대상 메서드에서 값 타입의 박싱

```prsm
@burst
func calculateForces(positions: NativeArray<Float3>, forces: NativeArray<Float3>) {
    for i in 0..positions.length {
        forces[i] = computeGravity(positions[i])
    }
}
```

```csharp
[BurstCompile]
public void calculateForces(NativeArray<float3> positions, NativeArray<float3> forces)
{
    for (int i = 0; i < positions.Length; i++)
    {
        forces[i] = computeGravity(positions[i]);
    }
}
```

## Part III — 패턴 편의 기능

### `state machine`

명명된 상태, 전이, 선택적 `enter`/`exit` 훅을 가진 유한 상태 머신을 선언합니다. 컴파일러가 enum, 현재 상태 필드, 전이 디스패처를 생성합니다.

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

```csharp
private enum AiState { Idle, Chase, Attack }
private AiState _aiState = AiState.Idle;

public void TransitionAiState(string eventName)
{
    var prev = _aiState;
    _aiState = (_aiState, eventName) switch
    {
        (AiState.Idle, "playerDetected") => AiState.Chase,
        (AiState.Chase, "playerLost") => AiState.Idle,
        (AiState.Chase, "inAttackRange") => AiState.Attack,
        (AiState.Attack, "attackDone") => AiState.Chase,
        _ => _aiState,
    };
    if (prev != _aiState) { _exitAiState(prev); _enterAiState(_aiState); }
}
```

### `command`

선택적 undo 지원과 가드를 가진 실행 가능 액션을 선언합니다. UI 바인딩, 입력 처리, 액션 리플레이에 사용됩니다. 컴파일러가 중첩된 `ICommand` 구현을 생성합니다.

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

```csharp
public class MoveUnitCommand : ICommand
{
    private UnitController _owner;
    private Vector3 _target;
    private Vector3 _prevPos;

    public bool CanExecute() => _owner.isAlive && !_owner.isStunned;
    public void Execute()
    {
        _prevPos = _owner.transform.position;
        _owner.transform.position = _target;
    }
    public void Undo() { _owner.transform.position = _prevPos; }
}
```

### `bind` (MVVM)

값이 변경될 때 자동으로 구독자에게 알리는 반응형 프로퍼티를 선언합니다. 컴파일러가 `INotifyPropertyChanged` 배관과 변경 감지 필드를 생성합니다.

```prsm
component PlayerHUD : MonoBehaviour {
    bind hp: Int = 100
    bind playerName: String = "Hero"

    serialize hpLabel: TextMeshProUGUI
    serialize nameLabel: TextMeshProUGUI

    awake {
        bind hp to hpLabel.text
        bind playerName to nameLabel.text
    }
}
```

```csharp
private int _hp = 100;
public int hp
{
    get => _hp;
    set
    {
        if (_hp != value)
        {
            _hp = value;
            OnPropertyChanged(nameof(hp));
        }
    }
}

public event PropertyChangedEventHandler PropertyChanged;
private void OnPropertyChanged(string name)
    => PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(name));
```

## Part IV — 개발자 경험

### 에러 메시지 개선

컴파일러 진단이 caret 라벨, help 텍스트, 노트를 포함한 Rust/Elm 스타일의 다중 줄 형식을 채택합니다.

```text
error[E090]: Interface member 'takeDamage' not implemented
  --> src/Enemy.prsm:15:1
   |
15 | component Enemy : MonoBehaviour, IDamageable {
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = help: Add the missing method:
   |   func takeDamage(amount: Int) {
   |       // TODO: implement
   |   }
   = note: Required by interface 'IDamageable' declared in src/IDamageable.prsm:3
```

렌더러는 또한 다음을 생성합니다:

- 잘못 표기된 식별자에 대한 Levenshtein 거리 기반 Did-you-mean 제안
- `toInt()`, `toFloat()` 또는 주석 변경을 제안하는 타입 불일치 도움말
- 일반적인 실수 감지 (`if hp = 0`은 E145를 발생시킴)

컴파일러가 색상 지원 터미널을 감지하면 ANSI 색상이 활성화됩니다.

### 리팩토링 도구

언어 서버는 다섯 가지 코드 액션을 노출합니다:

- **Extract Method** — 선택한 구문을 새 메서드로 추출하며 매개변수와 반환값을 자동으로 계산
- **Extract Component** — 선택한 필드와 메서드를 새 component 파일로 옮기고 `require` 참조로 대체
- **Inline Variable** — 단일 사용 `val`을 사용처에서 초기화 표현식으로 치환
- **Rename Symbol** — 프로젝트 전체에 걸쳐 선언과 모든 참조를 이름 변경
- **Convert to State Machine** — `enum` + `switch` 패턴을 감지하여 `state machine` 편의 기능으로 변환 제공

### 디버거 통합

컴파일러는 생성된 C# 파일과 함께 평탄한 `.prsm.map` 소스 맵을 출력하여 IDE에서 소스 수준 디버깅을 가능하게 합니다.

```json
{
    "version": 1,
    "source": "src/Player.prsm",
    "generated": "Generated/Player.cs",
    "mappings": [
        { "prsmLine": 5, "csLine": 12 },
        { "prsmLine": 6, "csLine": 13 },
        { "prsmLine": 10, "csLine": 20 }
    ]
}
```

디버거 어댑터는:

- `.prsm` 줄에 설정된 중단점을 해당 생성 `.cs` 줄로 변환
- 생성된 변수 이름 (`_prsm_d`, `__hp`)을 표시용 PrSM 식별자로 다시 매핑
- 스텝 진행 시 컴파일러 생성 보일러플레이트 (싱글턴 설정, 풀 초기화, 상태 머신 디스패치)를 건너뜀

`--debug-maps` 플래그가 생성을 제어합니다. 기본값은 Debug 빌드에서 `true`, Release 빌드에서 `false`입니다.

## 새로운 진단

### 오류

| 코드 | 기능 | 조건 |
|------|------|------|
| E100 | try/catch | 도달 불가능한 catch 절 (위에서 슈퍼타입이 잡힘) |
| E101 | throw | 표현식이 Exception 서브타입이 아님 |
| E102 | Lambda | 예상 함수 타입과 매개변수 개수 불일치 |
| E103 | Lambda | 컨텍스트 없이 매개변수 타입을 추론할 수 없음 |
| E104 | Lambda | 다중 매개변수 람다에 `it` 사용 |
| E105 | const | 비리터럴 초기화자 |
| E106 | static | 라이프사이클 블록에 `static` |
| E107 | Collection | 타입 주석 없는 빈 컬렉션 리터럴 |
| E108 | Collection | 혼합된 원소 타입 |
| E109 | Cast | 명백히 무관한 타입으로의 `as!` |
| E110 | Property | `val` 프로퍼티에 `set` |
| E111 | Property | 프로퍼티 접근자 외부의 `field` |
| E112 | extend | 확장이 기존 멤버와 충돌 |
| E113 | extend | 확장 블록이 상태를 포함 |
| E114 | Modifier | 일치하는 `open`/`abstract` 부모 없는 `override` |
| E115 | sealed | sealed 클래스에 비완전 `when` |
| E116 | abstract | 추상 클래스 인스턴스화 |
| E117 | Tuple | 구조 분해 개수 불일치 |
| E118 | Tuple | 잘못된 라벨로 명명 튜플 필드 접근 |
| E119 | use | 타입이 IDisposable을 구현하지 않음 |
| E120 | struct | 클래스를 상속하는 구조체 |
| E121 | event | 선언 타입 외부에서 이벤트 호출 |
| E122 | event | 비-Unit 반환 타입의 이벤트 |
| E123 | Operator | 알 수 없는 연산자 이름 |
| E124 | Operator | GetHashCode 없는 `operator equals` |
| E125 | Indexer | 가변 인덱서에 `operator set` 없는 `operator get` |
| E126 | typealias | 순환 타입 별칭 |
| E127 | typealias | 별칭이 내장 타입을 가림 |
| E128 | Raw string | 종료되지 않은 원시 문자열 |
| E129 | in | Contains/ContainsKey 없는 타입에 `in` |
| E130 | OR pattern | OR 패턴 분기가 다른 변수 바인딩 |
| E131 | Range pattern | 범위 low > high |
| E132 | ?:= | non-nullable에 null 병합 대입 |
| E133 | ?:= | `val`에 null 병합 대입 |
| E134 | Interface | 타깃 C# 버전에서 지원하지 않는 디폴트 메서드 |
| E135 | async | `async func` 외부의 `await` |
| E136 | async | awaitable이 아닌 반환 타입 |
| E137 | Burst | `@burst` 메서드의 관리 타입 |
| E138 | Burst | `@burst` 메서드의 try/catch |
| E139 | Burst | `@burst` 컨텍스트의 가상/인터페이스 호출 |
| E140 | State | 선언되지 않은 상태로의 전이 |
| E141 | State | 중복된 상태 이름 |
| E142 | Command | `undo`가 캡처되지 않은 변수 참조 |
| E143 | bind | bind 타깃이 쓰기 가능하지 않음 |
| E144 | bind | bind 타입 불일치 |
| E145 | DX | 조건에 사용된 대입 |
| E146 | DX | 일반적인 API 오용 |

### 경고

| 코드 | 기능 | 조건 |
|------|------|------|
| W020 | try/catch | 빈 catch 블록 |
| W021 | Cast | `as?` 결과가 null 검사되지 않음 |
| W022 | struct | 큰 구조체 (16바이트 초과) |
| W023 | Range | 겹치는 범위 패턴 |
| W024 | Interface | 디폴트 메서드가 인스턴스 상태 참조 |
| W025 | async | `async func`가 await를 사용하지 않음 |
| W026 | Optimizer | 핫 패스의 문자열 할당 |
| W027 | Optimizer | Update/FixedUpdate의 LINQ |
| W028 | Burst | `@burst` 메서드의 박싱 |
| W029 | State | 죽은 상태 (나가는 전이 없음) |
| W030 | Command | `canExecute` 가드 없는 command |
| W031 | bind | 읽히지 않는 bind 프로퍼티 |
| W032 | Debugger | 소스 맵 생성 실패 (치명적 아님) |

## 기능 게이트

언어 4의 모든 기능은 `version = "4"` 설정으로 암묵적으로 활성화됩니다. 개별 기능은 언어 3에서 선택적으로 활성화할 수 있습니다:

```toml
[language]
version = "3"
features = ["try-catch", "lambda", "static-const"]
```

| 플래그 | 설명 |
|--------|------|
| `try-catch` | try/catch/finally 와 throw |
| `lambda` | 람다식과 함수 타입 |
| `static-const` | static 및 const 멤버 |
| `collection-literals` | List, Map, Set 리터럴 |
| `type-cast` | as / as! / 스마트 캐스트 |
| `property` | field를 가진 프로퍼티 get/set |
| `extend` | 확장 메서드 |
| `class-modifiers` | abstract / sealed / open |
| `tuple` | 튜플 타입과 구조 분해 |
| `use` | IDisposable을 위한 use |
| `struct` | struct 선언 |
| `event` | event 멀티캐스트 델리게이트 |
| `operator` | 연산자 오버로딩 |
| `indexer` | 인덱서 (operator get/set) |
| `typealias` | typealias 선언 |
| `raw-string` | 원시 문자열 리터럴 |
| `in-operator` | in 멤버십 연산자 |
| `or-pattern` | when의 OR 패턴 |
| `range-pattern` | when의 범위 패턴 |
| `null-coalesce-assign` | ?:= 연산자 |
| `default-interface` | 디폴트 인터페이스 메서드 |
| `async-await` | async / await |
| `optimizer-v4` | v4 옵티마이저 규칙 (opt.string, opt.linq, opt.structcopy) |
| `burst-analysis` | Burst 호환성 분석 |
| `state-machine` | state machine 편의 기능 |
| `command` | command 편의 기능 |
| `bind` | bind 반응형 프로퍼티 |
| `error-enhancement` | Rust/Elm 스타일 진단 |
| `refactor-tools` | LSP 리팩토링 코드 액션 |
| `debugger` | 평탄한 소스 맵 생성 |

## 툴체인

- **Prism v2.0.0** — 언어 4 컴파일러, v4 LSP, v4 VS Code 확장을 번들
- **30개 기능** 6개 컴파일러 페이즈에 걸쳐 구현
- **385개 테스트** (lib 338 + 통합 47)가 v4 전체 표면을 커버
- **소스 맵** 은 IDE 중단점 매핑을 위해 Debug 빌드에서 기본 생성
- **UniTask** 는 패키지가 사용 가능할 때 비동기 변환에서 우선되며, 사용 불가 시 `Task`로 폴백
