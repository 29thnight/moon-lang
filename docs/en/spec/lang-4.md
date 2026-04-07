---
title: PrSM 4
parent: Specification
nav_order: 6
---

# PrSM Language 4

PrSM 4 is the largest single release of the language to date. It adds 30 features across four categories — syntax extensions, async + performance, pattern sugar, and developer experience — and ships as **Prism v2.0.0**. All Language 3 programs continue to compile without changes.

**Activation:** `language.version = "4"` in `.prsmproject`

## Part I — Syntax extensions

### `try` / `catch` / `finally` and `throw`

Exceptions are first-class. The `new` keyword is omitted on `throw`.

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

`try` is an expression when it has exactly one `catch` clause.

### Lambda expressions and function types

Lambdas are anonymous functions enclosed in `{ }`. A single-parameter lambda may use the implicit `it` identifier. When the last argument of a call is a lambda, it may be placed outside the parentheses (trailing lambda).

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

`(A, B) => R` lowers to `Func<A, B, R>` and `() => Unit` lowers to `Action`.

### `static` and `const`

`static` declares class-level members accessible without an instance. `const` declares compile-time constants restricted to literal initializers.

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

### Collection literals

Inline syntax for `List<T>`, `Map<K, V>`, `Array<T>`, and `Set<T>`.

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

Empty collection literals require an explicit type annotation (E107).

### Type casting

`as Type?` performs a safe cast that returns `null` on failure. `as! Type` performs a force cast that throws on failure. After an `is` check, the variable is smart-cast to the checked type within the same scope.

```prsm
val enemy = collider as Enemy?
val boss = collider as! Boss

if collider is BoxCollider {
    log(collider.size)   // smart-cast to BoxCollider
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

### Property `get` / `set`

Properties may have custom accessors. `val` with only `get` defines a computed read-only property. Inside `set`, the implicit `field` keyword refers to the auto-generated backing field.

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

### Extension methods (`extend`)

`extend Type { }` adds methods and computed properties to an existing type without modifying its source.

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

Lowers to a static class with C# extension methods. Extension blocks shall not contain mutable state (E113).

### `abstract` / `sealed` / `open`

By default, classes and methods are final. `open` permits inheritance and override. `abstract` requires a subclass implementation. `sealed` restricts subclasses to the same file, enabling exhaustive `when` checks.

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

### Tuples

Tuples group multiple values into a single compound value. Named tuples allow access by label. Tuples support destructuring.

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

### `use` for `IDisposable`

`use` ensures automatic disposal of `IDisposable` resources. The block form disposes at block exit; the declaration form disposes at the enclosing scope exit.

```prsm
use stream = FileStream(path, FileMode.Open) {
    val data = stream.readToEnd()
}

use val conn = DbConnection(connString)
// conn auto-disposed at scope end
```

```csharp
using (var stream = new FileStream(path, FileMode.Open))
{
    var data = stream.ReadToEnd();
}

using var conn = new DbConnection(connString);
```

### `struct` declaration

`struct` declares a value type with copy semantics and stack allocation.

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

Structs may not inherit from a class (E120). Large structs (over 16 bytes) emit W022.

### `event` (multicast delegates)

Function types `(A, B) => Unit` may be used directly as single-cast callbacks. The `event` keyword declares a multicast delegate that supports `+=` / `-=` and `.invoke()`.

```prsm
// Single-cast callback
var onDamaged: ((Int, DamageType) => Unit)? = null
onDamaged?.invoke(amount, type)

// Multicast event
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

### Operator overloading

Operator functions map symbolic operators to named methods, following Kotlin conventions: `plus`, `minus`, `times`, `div`, `mod`, `compareTo`, `equals`, `unaryMinus`, `not`.

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

### Indexer

`operator get` and `operator set` define indexer access using `[]` syntax.

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

`typealias` introduces a compile-time alias for an existing type. Aliases are erased during lowering.

```prsm
typealias Position = Vector3
typealias EnemyList = List<Enemy>

val pos: Position = vec3(1, 2, 3)
val enemies: EnemyList = []
```

### Raw string literals

Triple-quoted strings preserve newlines and special characters without escaping. Interpolation is still active.

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

Lowers to a C# 11 raw string literal where supported, falling back to `@"..."` verbatim strings for older targets.

### `in` operator

`in` tests membership against ranges, lists, and maps.

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

### OR patterns

Multiple patterns separated by commas in a `when` arm match if any individual pattern matches. All arms in an OR group must bind the same variables (or none).

```prsm
when direction {
    Direction.Up, Direction.Down    => handleVertical()
    Direction.Left, Direction.Right => handleHorizontal()
}
```

### Range patterns

`in low..high` inside a `when` arm matches values in the inclusive range `[low, high]`.

```prsm
when score {
    in 90..100 => "A"
    in 80..89  => "B"
    in 70..79  => "C"
    else       => "F"
}
```

### Null coalescing assignment (`?:=`)

`a ?:= b` assigns `b` to `a` only if `a` is currently `null`. The left-hand side must be a nullable mutable variable.

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

### Default interface methods

Interface methods may include a default body. Implementors may omit the method to use the default. Requires C# 8 or later default interface methods.

```prsm
interface IMovable {
    val speed: Float
    func move(direction: Vector3) {
        transform.translate(direction * speed * Time.deltaTime)
    }
}

component Player : MonoBehaviour, IMovable {
    val speed: Float = 5.0
    // move() uses the default implementation
}
```

## Part II — Async and performance

### `async` / `await`

`async func` declares an asynchronous function. `await` suspends until the awaited task completes. Inside Unity contexts the compiler prefers UniTask, falling back to `Task`.

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

### Optimizer enhancements

Three additional optimizer passes target Unity-specific performance patterns. They are activated by the same `--optimize` flag introduced in Language 3.

- **opt.string** — repeated `$"..."` expressions in `update`/`fixedUpdate`/`lateUpdate` are cached into a backing field with a change-detection guard. Allocations move from per-frame to per-change.
- **opt.linq** — simple LINQ chains on `List<T>` (`.Where(...).ToList()`, `.Select(...).ToList()`) inside hot paths are rewritten as manual `for` loops to eliminate enumerator allocation.
- **opt.structcopy** — large `Vector3`/`Quaternion`/`Matrix4x4` locals on hot paths are flagged for `ref readonly` substitution.

The optimizer emits W026 (string allocation cached) and W027 (LINQ rewritten) for affected sites so developers can audit each rewrite.

### Burst compatibility analysis

Static analysis detects whether a method or struct can be Burst-compiled. Reports:

- **E137** — managed type reference (class, string, delegate) in a Burst-targeted method
- **E138** — `try`/`catch` or `throw` in a Burst-targeted method
- **E139** — virtual or interface dispatch in a Burst context
- **W028** — boxing of a value type in a Burst-targeted method

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

## Part III — Pattern sugar

### `state machine`

Declares a finite state machine with named states, transitions, and optional `enter`/`exit` hooks. The compiler generates the enum, current-state field, and transition dispatcher.

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

Declares an executable action with optional undo support and a guard. Used for UI binding, input handling, and action replay. The compiler generates a nested `ICommand` implementation.

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

Declares a reactive property that automatically notifies subscribers when its value changes. The compiler generates `INotifyPropertyChanged` plumbing and change-detection fields.

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

## Part IV — Developer experience

### Error message enhancement

Compiler diagnostics adopt a Rust/Elm-style multi-line format with caret labels, help text, and notes.

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

The renderer also produces:

- Did-you-mean suggestions via Levenshtein-distance matching for misspelled identifiers
- Type-mismatch help text suggesting `toInt()`, `toFloat()`, or annotation changes
- Common-mistake detection (`if hp = 0` triggers E145)

ANSI color is enabled when the compiler detects a color-capable terminal.

### Refactoring tools

The Language Server exposes five code actions:

- **Extract Method** — extracts selected statements into a new method, computing parameters and return values automatically
- **Extract Component** — moves selected fields and methods into a new component file with a `require` reference
- **Inline Variable** — replaces a single-use `val` with its initializer at the usage site
- **Rename Symbol** — renames a declaration and all references across the project
- **Convert to State Machine** — detects `enum` + `switch` patterns and offers conversion to the `state machine` sugar

### Debugger integration

The compiler emits a flat `.prsm.map` source map alongside generated C# files, enabling source-level debugging in IDEs.

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

The debugger adapter:

- Translates breakpoints set on `.prsm` lines to the corresponding generated `.cs` line
- Maps generated variable names (`_prsm_d`, `__hp`) back to PrSM identifiers for display
- Skips compiler-generated boilerplate (singleton setup, pool init, state-machine dispatch) when stepping

The flag `--debug-maps` controls generation. The default is `true` in Debug builds and `false` in Release.

## New diagnostics

### Errors

| Code | Feature | Condition |
|------|---------|-----------|
| E100 | try/catch | Unreachable catch clause (supertype caught above) |
| E101 | throw | Expression is not an Exception subtype |
| E102 | Lambda | Parameter count mismatch with expected function type |
| E103 | Lambda | Cannot infer parameter types without context |
| E104 | Lambda | `it` used in multi-parameter lambda |
| E105 | const | Non-literal initializer |
| E106 | static | `static` on a lifecycle block |
| E107 | Collection | Empty collection literal without type annotation |
| E108 | Collection | Mixed element types |
| E109 | Cast | `as!` to provably unrelated type |
| E110 | Property | `set` on a `val` property |
| E111 | Property | `field` outside a property accessor |
| E112 | extend | Extension conflicts with existing member |
| E113 | extend | Extension block contains state |
| E114 | Modifier | `override` without matching `open`/`abstract` parent |
| E115 | sealed | Non-exhaustive `when` on sealed class |
| E116 | abstract | Instantiation of abstract class |
| E117 | Tuple | Destructuring count mismatch |
| E118 | Tuple | Named tuple field access with wrong label |
| E119 | use | Type does not implement IDisposable |
| E120 | struct | Struct inheriting from a class |
| E121 | event | Event invoked from outside declaring type |
| E122 | event | Event with non-Unit return type |
| E123 | Operator | Unknown operator name |
| E124 | Operator | `operator equals` without GetHashCode |
| E125 | Indexer | `operator get` without `operator set` on mutable indexer |
| E126 | typealias | Circular type alias |
| E127 | typealias | Alias shadows built-in type |
| E128 | Raw string | Unterminated raw string |
| E129 | in | `in` on type without Contains/ContainsKey |
| E130 | OR pattern | OR pattern arms bind different variables |
| E131 | Range pattern | Range low > high |
| E132 | ?:= | Null coalescing assign on non-nullable |
| E133 | ?:= | Null coalescing assign on `val` |
| E134 | Interface | Default method unsupported in target C# version |
| E135 | async | `await` outside `async func` |
| E136 | async | Non-awaitable return type |
| E137 | Burst | Managed type in `@burst` method |
| E138 | Burst | try/catch in `@burst` method |
| E139 | Burst | Virtual/interface call in `@burst` context |
| E140 | State | Transition to undeclared state |
| E141 | State | Duplicate state name |
| E142 | Command | `undo` references uncaptured variable |
| E143 | bind | Bind target not writable |
| E144 | bind | Bind type mismatch |
| E145 | DX | Assignment used as condition |
| E146 | DX | Common API misuse |

### Warnings

| Code | Feature | Condition |
|------|---------|-----------|
| W020 | try/catch | Empty catch block |
| W021 | Cast | `as?` result never null-checked |
| W022 | struct | Large struct (over 16 bytes) |
| W023 | Range | Overlapping range patterns |
| W024 | Interface | Default method references instance state |
| W025 | async | `async func` never awaits |
| W026 | Optimizer | String allocation in hot path |
| W027 | Optimizer | LINQ in Update/FixedUpdate |
| W028 | Burst | Boxing in `@burst` method |
| W029 | State | Dead state (no outgoing transitions) |
| W030 | Command | Command without `canExecute` guard |
| W031 | bind | Bind property never read |
| W032 | Debugger | Source map generation failed (non-fatal) |

## Feature gates

All Language 4 features are implicitly enabled by `version = "4"`. Individual features may be selectively enabled from Language 3:

```toml
[language]
version = "3"
features = ["try-catch", "lambda", "static-const"]
```

| Flag | Description |
|------|-------------|
| `try-catch` | try/catch/finally and throw |
| `lambda` | Lambda expressions and function types |
| `static-const` | static and const members |
| `collection-literals` | List, Map, Set literals |
| `type-cast` | as / as! / smart cast |
| `property` | Property get/set with field |
| `extend` | Extension methods |
| `class-modifiers` | abstract / sealed / open |
| `tuple` | Tuple types and destructuring |
| `use` | use for IDisposable |
| `struct` | struct declaration |
| `event` | event multicast delegates |
| `operator` | Operator overloading |
| `indexer` | Indexer (operator get/set) |
| `typealias` | typealias declaration |
| `raw-string` | Raw string literals |
| `in-operator` | in membership operator |
| `or-pattern` | OR patterns in when |
| `range-pattern` | Range patterns in when |
| `null-coalesce-assign` | ?:= operator |
| `default-interface` | Default interface methods |
| `async-await` | async / await |
| `optimizer-v4` | v4 optimizer rules (opt.string, opt.linq, opt.structcopy) |
| `burst-analysis` | Burst compatibility analysis |
| `state-machine` | state machine sugar |
| `command` | command sugar |
| `bind` | bind reactive properties |
| `error-enhancement` | Rust/Elm-style diagnostics |
| `refactor-tools` | LSP refactoring code actions |
| `debugger` | Flat source map generation |

## Toolchain

- **Prism v2.0.0** — bundles the Language 4 compiler, the v4 LSP, and the v4 VS Code extension
- **30 features** implemented across six compiler phases
- **385 tests** (338 lib + 47 integration) cover the v4 surface
- **Source maps** are generated by default in Debug builds for IDE breakpoint mapping
- **UniTask** is preferred for async lowering when the package is available; the compiler falls back to `Task` otherwise
