# PrSM 미지원 문법 전체 분석 — Kotlin-like 표현 설계

**목적:** 현재 PrSM에서 `intrinsic` 없이 표현할 수 없는 C# 기능을 열거하고, 각각을 PrSM의 Kotlin-like 문법으로 어떻게 녹여낼지 설계한다.

---

## 1. 에러 처리

### 1.1 try/catch/finally

**C#:**
```csharp
try {
    var data = File.ReadAllText(path);
} catch (FileNotFoundException e) {
    Debug.LogWarning(e.Message);
} catch (Exception e) {
    Debug.LogError(e.Message);
} finally {
    Cleanup();
}
```

**PrSM (Kotlin-like):**
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
```

Kotlin과 동일. C#과 1:1 매핑.

### 1.2 throw

**C#:**
```csharp
throw new ArgumentException("Invalid value");
```

**PrSM:**
```prsm
throw ArgumentException("Invalid value")
```

`new` 키워드 생략 (Kotlin 스타일). 생성 C#에서 `new` 자동 삽입.

### 1.3 try as expression

**Kotlin:**
```kotlin
val result = try { parseInt(str) } catch (e: Exception) { -1 }
```

**PrSM:**
```prsm
val result = try { parseInt(str) } catch (e: Exception) { -1 }
```

try도 expression으로 사용 가능. if/when과 동일한 패턴.

---

## 2. Lambda / 클로저 / 함수 타입

### 2.1 Lambda 표현식

**C#:**
```csharp
Action<int> callback = (x) => Debug.Log(x);
Func<int, int, int> add = (a, b) => a + b;
list.Where(x => x > 10).Select(x => x * 2);
```

**PrSM:**
```prsm
val callback: (Int) => Unit = { x => log(x) }
val add: (Int, Int) => Int = { a, b => a + b }
list.where({ x => x > 10 }).select({ x => x * 2 })
```

### 2.2 후행 Lambda (Kotlin trailing lambda)

**Kotlin:**
```kotlin
list.filter { it > 10 }
button.setOnClickListener { view -> handleClick(view) }
```

**PrSM:**
```prsm
list.filter { it > 10 }
button.onClick { view => handleClick(view) }
```

마지막 파라미터가 lambda면 `()` 바깥으로 빼낼 수 있음. 단일 파라미터는 `it`으로 참조.

### 2.3 함수 타입 선언

**C#:**
```csharp
public void DoWork(Action<int> callback) { }
public Func<int, bool> GetPredicate() { }
```

**PrSM:**
```prsm
func doWork(callback: (Int) => Unit) { }
func getPredicate(): (Int) => Bool { }
```

`(ParamTypes) => ReturnType` 문법. C#의 `Action`/`Func`으로 lowering.

### 2.4 클로저 캡처

**PrSM:**
```prsm
var counter = 0
val increment = { counter += 1 }
increment()
increment()
log(counter) // 2
```

C# lambda와 동일한 캡처 의미론. 변수 참조 캡처.

---

## 3. Static / Const / Companion

### 3.1 static 멤버

**C#:**
```csharp
public class MathHelper {
    public static float PI = 3.14159f;
    public static float Lerp(float a, float b, float t) => a + (b - a) * t;
}
```

**PrSM (방안 A — static 키워드):**
```prsm
class MathHelper {
    static val PI: Float = 3.14159
    static func lerp(a: Float, b: Float, t: Float): Float = a + (b - a) * t
}
```

**PrSM (방안 B — companion object, Kotlin 스타일):**
```prsm
class MathHelper {
    companion {
        val PI: Float = 3.14159
        func lerp(a: Float, b: Float, t: Float): Float = a + (b - a) * t
    }
}
```

**권장: 방안 A** — C# lowering이 직관적이고, 학습 곡선이 낮음.

### 3.2 const

**C#:**
```csharp
public const int MAX_HEALTH = 100;
public const string VERSION = "1.0.0";
```

**PrSM:**
```prsm
const MAX_HEALTH: Int = 100
const VERSION: String = "1.0.0"
```

`const`는 이미 렉서에 토큰이 존재함. 컴파일 타임 상수만 허용 (리터럴, 문자열, 숫자).

### 3.3 component의 static

```prsm
component ScoreManager : MonoBehaviour {
    static var highScore: Int = 0

    func addScore(amount: Int) {
        ScoreManager.highScore += amount
    }
}
```

생성 C#: `public static int highScore = 0;`

---

## 4. 컬렉션 리터럴

### 4.1 List 리터럴

**C#:**
```csharp
var numbers = new List<int> { 1, 2, 3, 4, 5 };
```

**PrSM:**
```prsm
val numbers = [1, 2, 3, 4, 5]
val names = ["Alice", "Bob", "Charlie"]
val empty: List<Int> = []
```

`[elements]`는 `List<T>`로 추론. 빈 리스트는 타입 표기 필수.

### 4.2 Map 리터럴

**C#:**
```csharp
var lookup = new Dictionary<string, int> { {"hp", 100}, {"mp", 50} };
```

**PrSM:**
```prsm
val lookup = {"hp": 100, "mp": 50}
val emptyMap: Map<String, Int> = {}
```

`{key: value}` 형식. JSON 친화적.

### 4.3 Array 리터럴

**PrSM:**
```prsm
val arr: Array<Int> = arrayOf(1, 2, 3)
```

`arrayOf()` sugar. `T[]`로 lowering.

### 4.4 Set 리터럴

**PrSM:**
```prsm
val uniqueNames = setOf("Alice", "Bob", "Alice") // {"Alice", "Bob"}
```

`setOf()` sugar. `HashSet<T>`로 lowering.

---

## 5. 타입 캐스팅 / 변환

### 5.1 안전 캐스트 (as)

**C#:**
```csharp
var enemy = collider as Enemy; // null if fails
```

**PrSM:**
```prsm
val enemy = collider as Enemy?  // null if fails
```

`as Type?` — nullable 반환. C#의 `as` 그대로.

### 5.2 강제 캐스트 (as!)

**C#:**
```csharp
var enemy = (Enemy)collider; // throws InvalidCastException
```

**PrSM:**
```prsm
val enemy = collider as! Enemy  // throws if fails
```

`as!` — non-null 단언 캐스트. Kotlin의 `as`에 해당. `!!`과 유사한 느낌.

### 5.3 스마트 캐스트

**Kotlin:**
```kotlin
if (obj is String) {
    println(obj.length) // obj is automatically cast to String
}
```

**PrSM:**
```prsm
if collider is BoxCollider {
    log(collider.size) // collider가 자동으로 BoxCollider로 좁혀짐
}

when target {
    is Enemy => target.takeDamage(10)  // target이 Enemy로 좁혀짐
    is Ally => target.heal(5)
}
```

`is` 체크 후 블록 내에서 자동 타입 축소. 컴파일러가 C# pattern matching으로 lowering.

### 5.4 숫자 타입 변환

**C#:**
```csharp
int i = 42;
float f = (float)i;
double d = (double)f;
```

**PrSM:**
```prsm
val i: Int = 42
val f = i.toFloat()
val d = f.toDouble()
val s = i.toString()
```

Kotlin 스타일 변환 메서드. `toFloat()`, `toInt()`, `toDouble()`, `toString()` 등.
Sugar로 lowering: `(float)i`, `(double)f`, `i.ToString()`.

---

## 6. Property (get/set)

### 6.1 커스텀 getter/setter

**C#:**
```csharp
private int _hp;
public int Hp {
    get => _hp;
    set {
        _hp = Mathf.Clamp(value, 0, maxHp);
        OnHpChanged?.Invoke(_hp);
    }
}
```

**PrSM:**
```prsm
var hp: Int
    get = _hp
    set(value) {
        _hp = Mathf.clamp(value, 0, maxHp)
        onHpChanged?.invoke(_hp)
    }
```

Kotlin 스타일. `get`/`set` 블록은 선택적. 없으면 기본 backing field 사용.

### 6.2 computed property (getter only)

**PrSM:**
```prsm
val isAlive: Bool
    get = hp > 0

val displayName: String
    get = "$firstName $lastName"
```

`val` + `get`만 있으면 readonly computed property. C#의 `=> hp > 0` 프로퍼티로 lowering.

### 6.3 backing field

**PrSM:**
```prsm
var name: String = "Default"
    set(value) {
        field = value.trim()  // 'field'는 자동 backing field (Kotlin과 동일)
        log("Name changed to $field")
    }
```

Kotlin의 `field` 키워드로 backing field 접근.

---

## 7. 확장 함수 (Extension Methods)

### 7.1 기본 확장

**C#:**
```csharp
public static class TransformExtensions {
    public static void ResetLocal(this Transform t) {
        t.localPosition = Vector3.zero;
        t.localRotation = Quaternion.identity;
        t.localScale = Vector3.one;
    }
}
```

**PrSM:**
```prsm
extend Transform {
    func resetLocal() {
        this.localPosition = Vector3.zero
        this.localRotation = Quaternion.identity
        this.localScale = Vector3.one
    }
}

// 사용
transform.resetLocal()
```

`extend Type { }` 블록. Kotlin의 extension function보다 그룹화에 유리.

### 7.2 확장 프로퍼티

**PrSM:**
```prsm
extend Vector3 {
    val flat: Vector3
        get = vec3(this.x, 0, this.z)
}

// 사용
val groundPos = transform.position.flat
```

---

## 8. Delegate / Event 선언

### 8.1 delegate 타입

**C#:**
```csharp
public delegate void DamageHandler(int amount, DamageType type);
public event DamageHandler OnDamaged;
```

**PrSM:**
```prsm
// 함수 타입으로 대체 (Kotlin 스타일)
var onDamaged: ((Int, DamageType) => Unit)? = null

// 발행
onDamaged?.invoke(amount, type)

// 구독
enemy.onDamaged = { amount, type => log("Hit for $amount") }
```

Kotlin처럼 별도 delegate 키워드 없이 함수 타입으로 처리. C#의 `Action<int, DamageType>`으로 lowering.

### 8.2 event 키워드 (멀티캐스트)

```prsm
event onDamaged: (Int, DamageType) => Unit

// 구독 추가
onDamaged += { amount, type => log("Hit!") }

// 발행
onDamaged.invoke(amount, type)
```

`event` 키워드는 C#의 `event`로 lowering. `+=`/`-=` 연산자 지원.

---

## 9. Abstract / Sealed / Open

### 9.1 abstract class

**C#:**
```csharp
public abstract class Weapon {
    public abstract void Attack();
    public virtual void Reload() { }
}
```

**PrSM:**
```prsm
abstract class Weapon {
    abstract func attack()
    open func reload() { }
}

class Sword : Weapon {
    override func attack() { swing() }
}
```

`abstract` — 반드시 override. `open` — override 가능 (기본은 final). Kotlin과 동일한 모델.

### 9.2 sealed class

**PrSM:**
```prsm
sealed class Shape {
    class Circle(radius: Float) : Shape
    class Rect(width: Float, height: Float) : Shape
    class Triangle(base: Float, height: Float) : Shape
}

// when에서 exhaustive check
when shape {
    is Shape.Circle => area = PI * shape.radius * shape.radius
    is Shape.Rect => area = shape.width * shape.height
    is Shape.Triangle => area = shape.base * shape.height / 2
}
```

sealed class의 when은 `else` 없이도 완전성 검사 통과. ADT(대수 데이터 타입) 표현.

---

## 10. using 문 (IDisposable)

**C#:**
```csharp
using (var stream = new FileStream(path, FileMode.Open)) {
    // auto-disposed
}
// C# 8:
using var stream = new FileStream(path, FileMode.Open);
```

**PrSM:**
```prsm
use stream = FileStream(path, FileMode.Open) {
    val data = stream.readToEnd()
}

// 또는 스코프 변수 (C# 8 스타일)
use val stream = FileStream(path, FileMode.Open)
// 스코프 끝에서 자동 dispose
```

`use` 키워드. `using`은 이미 namespace import에 사용 중이므로 `use`로 차별화.

---

## 11. Indexer

**C#:**
```csharp
public int this[int index] {
    get => items[index];
    set => items[index] = value;
}
```

**PrSM:**
```prsm
class Inventory {
    var items: List<Item> = []

    operator get(index: Int): Item = items[index]
    operator set(index: Int, value: Item) { items[index] = value }
}

// 사용
val sword = inventory[0]
inventory[1] = shield
```

`operator get`/`operator set`으로 인덱서 정의.

---

## 12. Operator Overloading

**C#:**
```csharp
public static Vector2i operator +(Vector2i a, Vector2i b) => new(a.x + b.x, a.y + b.y);
```

**PrSM:**
```prsm
data class Vec2i(x: Int, y: Int) {
    operator plus(other: Vec2i): Vec2i = Vec2i(x + other.x, y + other.y)
    operator minus(other: Vec2i): Vec2i = Vec2i(x - other.x, y - other.y)
    operator times(scalar: Int): Vec2i = Vec2i(x * scalar, y * scalar)
}

// 사용
val a = Vec2i(1, 2)
val b = Vec2i(3, 4)
val c = a + b  // Vec2i(4, 6)
```

Kotlin 스타일 `operator` 함수. `plus`, `minus`, `times`, `div`, `mod`, `compareTo`, `equals`.

---

## 13. Struct (값 타입)

**C#:**
```csharp
public struct DamageInfo {
    public int amount;
    public DamageType type;
}
```

**PrSM:**
```prsm
struct DamageInfo(amount: Int, type: DamageType)
```

`data class`와 유사하지만 값 타입 (스택 할당). C#의 `struct`로 lowering. 복사 의미론.

**확장:**
```prsm
struct Color32(r: Byte, g: Byte, b: Byte, a: Byte) {
    static val white = Color32(255, 255, 255, 255)
    static val black = Color32(0, 0, 0, 255)
}
```

---

## 14. Tuple

**C#:**
```csharp
(int, string) GetResult() => (42, "answer");
var (num, name) = GetResult();
```

**PrSM:**
```prsm
func getResult(): (Int, String) = (42, "answer")
val (num, name) = getResult()
```

`(Type, Type)` 튜플 타입. C# ValueTuple로 lowering. 구조 분해와 자연스럽게 연계.

**이름 있는 튜플:**
```prsm
func getStats(): (hp: Int, mp: Int) = (hp: 100, mp: 50)
val stats = getStats()
log(stats.hp)
```

---

## 15. String 추가 기능

### 15.1 Raw string (여러 줄)

**PrSM:**
```prsm
val json = """
{
    "name": "Player",
    "level": 42
}
"""
```

`"""` triple-quote. C#의 raw string literal `"""` (C# 11)로 lowering.
Unity 2022.3 기본 C# 버전에서는 `@""` verbatim string으로 폴백.

### 15.2 문자열 곱셈

**PrSM:**
```prsm
val divider = "-" * 20  // "--------------------"
```

Sugar. `string.Concat(Enumerable.Repeat("-", 20))`으로 lowering.

---

## 16. 범위 / 패턴 확장

### 16.1 in 연산자

**PrSM:**
```prsm
if x in 1..10 { log("In range") }
if name in ["Alice", "Bob"] { log("Known user") }
if key in lookup { log("Key exists") }
```

C# lowering: `x >= 1 && x <= 10`, `list.Contains(name)`, `dict.ContainsKey(key)`.

### 16.2 when with multiple patterns (OR pattern)

**PrSM:**
```prsm
when direction {
    Direction.Up, Direction.Down => handleVertical()
    Direction.Left, Direction.Right => handleHorizontal()
}
```

쉼표로 여러 패턴 결합. C#의 `case Direction.Up: case Direction.Down:`으로 lowering.

### 16.3 범위 패턴

**PrSM:**
```prsm
when score {
    in 90..100 => "A"
    in 80..89 => "B"
    in 70..79 => "C"
    else => "F"
}
```

`in range` 패턴. C#의 `>= 90 && <= 100`으로 lowering.

---

## 17. 기타

### 17.1 null 병합 대입

**C#:**
```csharp
_instance ??= FindFirstObjectByType<T>();
```

**PrSM:**
```prsm
_instance ?:= FindFirstObjectByType<T>()
```

`?:=` 연산자. 좌변이 null일 때만 대입.

### 17.2 타입 별칭

**PrSM:**
```prsm
typealias Position = Vector3
typealias EnemyList = List<Enemy>

val pos: Position = vec3(1, 2, 3)
val enemies: EnemyList = []
```

컴파일 타임 별칭. lowering 시 원본 타입으로 치환.

### 17.3 디폴트 구현이 있는 인터페이스

**PrSM:**
```prsm
interface IMovable {
    val speed: Float
    func move(direction: Vector3) {
        // 디폴트 구현
        transform.translate(direction * speed * Time.deltaTime)
    }
}
```

C# 8 default interface method로 lowering. Unity의 C# 버전 지원 여부에 따라.

### 17.4 destructuring 확장 (N개 변수)

**PrSM:**
```prsm
val (x, y, z) = transform.position  // Vector3 구조 분해
val (r, g, b, a) = spriteRenderer.color  // Color 구조 분해
```

C#의 Deconstruct 메서드를 이용한 구조 분해. 확장 함수로 Deconstruct를 정의 가능.

---

## 요약: 우선순위별 분류

### 반드시 필요 (intrinsic 의존도 직접 감소)

| 기능 | 현재 대안 | 복잡도 |
|------|----------|--------|
| try/catch/finally | intrinsic | 낮음 |
| lambda/클로저 | listen 내부에서만 | 중간 |
| static + const | intrinsic | 낮음 |
| 컬렉션 리터럴 | intrinsic | 중간 |
| 타입 캐스팅 (as/as!) | intrinsic | 낮음 |
| throw | intrinsic | 낮음 |

### 매우 유용 (코드 품질 향상)

| 기능 | 복잡도 |
|------|--------|
| 커스텀 property (get/set) | 중간 |
| 확장 함수 (extend) | 중간 |
| abstract/sealed/open | 중간 |
| tuple 반환 | 낮음 |
| use (IDisposable) | 낮음 |
| 스마트 캐스트 | 중간 |

### 있으면 좋은 (편의 기능)

| 기능 | 복잡도 |
|------|--------|
| operator overloading | 중간 |
| struct (값 타입) | 중간 |
| typealias | 낮음 |
| raw string (""") | 낮음 |
| in 연산자 | 낮음 |
| OR 패턴 | 낮음 |
| null 병합 대입 (?:=) | 낮음 |
| event 키워드 | 중간 |
| indexer (operator get/set) | 중간 |
| 디폴트 인터페이스 구현 | 높음 |
