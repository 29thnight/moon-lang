# Moon Language Specification — v0 Design Package

**Status:** Draft v0.1
**Date:** 2026-04-03
**Target:** Unity 2022.3+ (IL2CPP / Mono)
**Backend:** Transpiles to C# source files consumed by Unity's compilation pipeline

---

## 1. Language Vision and Non-Goals

### Vision

Moon is a **Unity-first scripting language** with Kotlin-flavored syntax. It exists to make Unity gameplay programming **safer, more concise, and less ceremonial** than raw C# while remaining a **zero-cost abstraction** — every Moon program lowers to idiomatic C# that Unity developers could have written by hand.

**Core design principles:**

1. **Unity is the runtime.** Moon does not define its own runtime, GC, or object model. Unity's MonoBehaviour lifecycle, serialization, and ECS conventions are first-class language constructs, not library wrappers.
2. **Null-safety by default.** The type system distinguishes nullable from non-null references at compile time. Inspector-injected fields use `require` semantics with validation at `Awake()`.
3. **Kotlin familiarity, not Kotlin compatibility.** Syntax borrows from Kotlin where it reduces ceremony (expression bodies, `val`/`var`, `when`, null-safe operators). It diverges wherever Unity semantics demand it.
4. **Transparent lowering.** A Moon author can predict the C# output. No hidden allocations, no implicit coroutine state machines beyond what Unity already uses, no reflection at runtime.
5. **Progressive disclosure.** Simple scripts look simple. Advanced interop (native arrays, Jobs, custom editors) is possible but not required for the common case.

### Non-Goals

| Non-goal | Rationale |
|---|---|
| General-purpose language | Moon targets Unity scripting only. It is not a replacement for C# outside Unity. |
| Custom runtime / VM | All execution flows through Unity's Mono/IL2CPP pipeline via generated C#. |
| Multi-platform standard library | The standard library IS Unity's API surface. No wrapping layer. |
| Source-level C# interop (embedding C# in Moon) | Moon files are standalone. C# interop is assembly-level: Moon calls C# types, C# calls generated Moon types. |
| Metaprogramming / macros | Deferred indefinitely. Compile-time code generation is the transpiler's job. |
| Operator overloading | Deferred. Adds complexity to type inference and lowering. |
| Inheritance hierarchies for components | Components use composition via Unity's model. `class` supports single inheritance for non-component utility types only. |
| Async/await (C# style) | Unity's coroutine model is the concurrency primitive. Async may be added post-v0 for UniTask interop. |

---

## 2. Syntax Summary

Moon uses **significant braces** (not indentation), **no semicolons** (newline is statement terminator; line continuation via trailing operator or `\`), and **Kotlin-style declarations**.

```
// Single-line comment
/* Multi-line comment */

// Top-level declarations
component PlayerController { ... }
asset PlayerStats { ... }
class DamageCalculator { ... }
enum WeaponType { ... }

// Fields
val speed: Float = 5.0f          // immutable, serialized by default
var health: Int = 100             // mutable, serialized by default
@serialize val nickname: String   // explicit serialize on val
require rb: Rigidbody             // Inspector-injected, validated at Awake
optional audioSrc: AudioSource    // Inspector-injected, nullable
child healthBar: HealthBar        // GetComponentInChildren
parent manager: GameManager       // GetComponentInParent

// Functions
func takeDamage(amount: Int) { ... }
func isAlive(): Boolean = health > 0   // expression body

// Lifecycle
awake { rb = GetComponent<Rigidbody>() }  // lowered into Awake()
update { move(Time.deltaTime) }           // lowered into Update()

// Coroutines
coroutine fadeOut() {
    wait(1.0f)
    wait(until = { alpha <= 0f })
    wait(frames = 1)
}

// Control flow
if (x > 0) { ... } else { ... }
when (state) {
    State.Idle -> idle()
    State.Running -> run()
    else -> stop()
}
for (enemy in enemies) { ... }
for (i in 0 until 10) { ... }
while (alive) { ... }
```

---

## 3. Core Declarations

### 3.1 `component`

**Syntax:**
```
component <Name> {
    <members>
}
```

**Semantics:**
A `component` is the primary Moon declaration. It represents a Unity MonoBehaviour. There is no explicit base class — all components implicitly extend `MonoBehaviour`.

**Lowering intent:**
```csharp
// Moon
component Player { ... }

// C# output
public class Player : MonoBehaviour { ... }
```

**Unity rationale:**
MonoBehaviour is the fundamental unit of Unity scripting. Making it a keyword rather than inheritance syntax removes ceremony and prevents errors (forgetting to extend MonoBehaviour, accidentally extending the wrong base).

**Rules:**
- A component cannot extend another component (no MonoBehaviour inheritance chains — this is a Unity anti-pattern).
- A component cannot be instantiated with a constructor. Unity manages lifecycle.
- A component can implement interfaces (syntax: `component Player : IDamageable { ... }`).
- Exactly one component per file. File name must match component name.
- Components may not appear inside other declarations (no nesting).

**Examples:**
```
component Player : IDamageable {
    var health: Int = 100

    update {
        move()
    }

    func move() {
        transform.Translate(Vector3.forward * Time.deltaTime)
    }

    override func takeDamage(amount: Int) {
        health -= amount
    }
}
```

**Edge cases:**
- Declaring a constructor → compile error: "Components are managed by Unity and cannot have constructors."
- Extending another component → compile error: "Component inheritance is not supported. Use composition or interfaces."
- Multiple components in one file → compile error: "Only one component declaration per file."

**v0 status:** **Included.**

---

### 3.2 `asset`

**Syntax:**
```
asset <Name> {
    <fields>
    <functions>
}
```

**Semantics:**
An `asset` is a Unity ScriptableObject. It holds serializable data and optional pure functions. It has no lifecycle blocks (no `update`, `awake`, etc.).

**Lowering intent:**
```csharp
// Moon
asset WeaponData {
    val damage: Int = 10
    val range: Float = 5.0f
    func dps(): Float = damage / attackSpeed
}

// C# output
[CreateAssetMenu(fileName = "New WeaponData", menuName = "Moon/WeaponData")]
public class WeaponData : ScriptableObject {
    [SerializeField] private int _damage = 10;
    public int damage => _damage;
    [SerializeField] private float _range = 5.0f;
    public float range => _range;
    public float dps() => damage / attackSpeed;
}
```

**Unity rationale:**
ScriptableObjects are Unity's primary data container pattern. The `[CreateAssetMenu]` attribute is generated automatically so assets are immediately usable from the Unity editor.

**Rules:**
- No lifecycle blocks allowed.
- Fields follow the same `val`/`var`/`serialize` rules as components.
- No `require`/`optional`/`child`/`parent` field qualifiers (these are MonoBehaviour-specific).
- One `asset` per file. File name must match.

**Edge cases:**
- Using `update` inside an asset → compile error: "Lifecycle blocks are only valid inside component declarations."
- Using `require` inside an asset → compile error: "'require' fields are only valid inside component declarations."

**v0 status:** **Included.**

---

### 3.3 `class`

**Syntax:**
```
class <Name> {
    <members>
}

class <Name>(<constructor-params>) {
    <members>
}

class <Name> : <BaseClass>, <Interface> {
    <members>
}
```

**Semantics:**
A `class` is a plain C# class. It supports constructors, single inheritance, and interface implementation. Classes are for non-Unity utility types: damage calculators, state machines, data structures.

**Lowering intent:**
```csharp
// Moon
class DamageResult(val amount: Int, val wasCritical: Boolean)

// C# output
public class DamageResult {
    public int amount { get; }
    public bool wasCritical { get; }
    public DamageResult(int amount, bool wasCritical) {
        this.amount = amount;
        this.wasCritical = wasCritical;
    }
}
```

**Rules:**
- Classes CAN have constructors (unlike components).
- Primary constructor syntax (params in declaration header) generates backing fields.
- Single inheritance only. Multiple interface implementation allowed.
- No lifecycle blocks.
- No `require`/`optional`/`child`/`parent`.
- Classes are `public` by default. (Visibility modifiers deferred to post-v0.)
- No `abstract` or `sealed` modifiers in v0. Deferred.

**Edge cases:**
- Class extending a component → compile error: "Classes cannot extend components. Use 'component' declaration instead."
- Class with lifecycle block → compile error: "Lifecycle blocks are only valid inside component declarations."

**v0 status:** **Included.** Abstract/sealed deferred.

---

### 3.4 `enum`

**Syntax:**
```
enum <Name> {
    <Entry>,
    <Entry>,
    ...
}

enum <Name>(val <param>: <Type>) {
    <Entry>(<value>),
    ...
}
```

**Semantics:**
Simple enums lower to C# enums. Parameterized enums lower to C# enums with extension methods or a sealed class hierarchy (decision: use C# enum + companion object pattern for v0 simplicity).

**Lowering intent:**
```csharp
// Moon — simple
enum Direction {
    Up, Down, Left, Right
}

// C# output
public enum Direction { Up, Down, Left, Right }

// Moon — parameterized
enum Weapon(val damage: Int) {
    Sword(10),
    Bow(7),
    Staff(15)
}

// C# output
public enum Weapon { Sword, Bow, Staff }
public static class WeaponExtensions {
    public static int damage(this Weapon self) => self switch {
        Weapon.Sword => 10,
        Weapon.Bow => 7,
        Weapon.Staff => 15,
        _ => throw new System.ArgumentOutOfRangeException()
    };
}
```

**Rules:**
- Simple enums are plain C# enums. They support integer backing values via `= N`.
- Parameterized enums generate an extension method per parameter.
- Enum entries are comma-separated. Trailing comma allowed.
- `when` exhaustiveness checking applies to enums (see §8.2).

**Edge cases:**
- Parameterized enum with mutable param (`var`) → compile error: "Enum parameters must be 'val'."
- Empty enum → compile error: "Enum must have at least one entry."
- Duplicate entry names → compile error.

**v0 status:** **Included.** Sealed-class-style algebraic enums (Kotlin `sealed class`) deferred.

---

## 4. Field Model

### 4.1 `val` / `var`

**Syntax:**
```
val <name>: <Type> = <initializer>
var <name>: <Type> = <initializer>
val <name>: <Type>                    // must be assigned in awake or init
var <name>: <Type>                    // must be assigned before use
```

**Semantics:**

| Qualifier | Mutability | Serialized by default? | C# lowering |
|---|---|---|---|
| `val` | Immutable after initialization | Yes (in components/assets) | `[SerializeField] private T _field; public T field => _field;` |
| `var` | Mutable | Yes (in components/assets) | `[SerializeField] public T field;` |

**Key rule:** In components and assets, `val` and `var` fields of **serializable types** are serialized by default. This matches Unity convention — if it's on a MonoBehaviour, the designer probably wants to see it in the Inspector.

In `class` declarations, fields are NOT serialized by default (plain C# fields).

**Type inference:** The type annotation may be omitted when an initializer is present.
```
val speed = 5.0f    // inferred as Float
var name = "Player" // inferred as String
```

Type inference is **local only** — it works within a statement. Field types in public API positions (function return types, `require` fields without initializers) must be annotated explicitly.

**Lowering for `val` in components:**
```csharp
// Moon
component Foo {
    val speed: Float = 5.0f
}

// C# — val becomes a SerializeField with read-only public access
public class Foo : MonoBehaviour {
    [SerializeField] private float _speed = 5.0f;
    public float speed => _speed;
}
```

This allows designers to set values in the Inspector while preventing runtime code from accidentally mutating them.

**Lowering for `var` in components:**
```csharp
// Moon
component Foo {
    var health: Int = 100
}

// C#
public class Foo : MonoBehaviour {
    [SerializeField] public int health = 100;
}
```

**Edge cases:**
- `val` assigned in both initializer and `awake` → compile error: "val fields with initializers cannot be reassigned."
- `val` without initializer and not assigned in `awake` → compile error: "val field 'x' must be assigned in its declaration or in 'awake'."
- `var` without initializer read before assignment → compile error: "Variable 'x' may not have been initialized."
- Non-serializable type with `val`/`var` in component → field is still declared but NOT serialized; compiler emits warning: "Field 'x' of type 'T' is not serializable by Unity and will not appear in the Inspector."

**v0 status:** **Included.**

---

### 4.2 `@serialize`

**Syntax:**
```
@serialize val <name>: <Type> = <value>
@serialize var <name>: <Type> = <value>
```

**Semantics:**
Explicit serialization annotation. In components/assets, this is redundant (fields serialize by default) but serves as documentation. In `class` bodies, `@serialize` has no effect (classes don't participate in Unity serialization in v0).

The inverse, `@noserialize`, suppresses serialization:
```
@noserialize var internalCounter: Int = 0  // not visible in Inspector
```

**Lowering:**
- `@serialize` → `[SerializeField]`
- `@noserialize` → no `[SerializeField]`, field is `private` (or `public` with `[System.NonSerialized]`)

**v0 status:** **Included.**

---

### 4.3 `require` / `optional` / `child` / `parent`

These are **component-only field qualifiers** for Unity dependency injection patterns.

#### `require`

**Syntax:**
```
require <name>: <Type>
```

**Semantics:**
Declares a non-null reference to a Unity component that MUST be present. Validated at `Awake()` time. If missing, logs an error and disables the component.

**Lowering:**
```csharp
// Moon
component Player {
    require rb: Rigidbody
}

// C#
public class Player : MonoBehaviour {
    private Rigidbody _rb;
    public Rigidbody rb => _rb;

    private void Awake() {
        _rb = GetComponent<Rigidbody>();
        if (_rb == null) {
            Debug.LogError($"[Player] Required component Rigidbody not found on {gameObject.name}", this);
            enabled = false;
            return;
        }
    }
}
```

**Unity rationale:**
`GetComponent` + null check in `Awake` is the most common Unity boilerplate. `require` eliminates it and makes the dependency contract explicit.

#### `optional`

**Syntax:**
```
optional <name>: <Type>
```

**Semantics:**
Declares a nullable reference to a Unity component. Fetched at `Awake()` but failure is not an error. The field type is implicitly `T?`.

**Lowering:**
```csharp
private AudioSource _audioSrc;
public AudioSource audioSrc => _audioSrc; // may be null

private void Awake() {
    _audioSrc = GetComponent<AudioSource>();
}
```

#### `child`

**Syntax:**
```
child <name>: <Type>
```

**Semantics:**
Like `require`, but uses `GetComponentInChildren<T>()`.

#### `parent`

**Syntax:**
```
parent <name>: <Type>
```

**Semantics:**
Like `require`, but uses `GetComponentInParent<T>()`.

**Validation table:**

| Qualifier | Fetch method | Null behavior | Field nullability |
|---|---|---|---|
| `require` | `GetComponent<T>()` | Error + disable | Non-null (`T`) |
| `optional` | `GetComponent<T>()` | Silent | Nullable (`T?`) |
| `child` | `GetComponentInChildren<T>()` | Error + disable | Non-null (`T`) |
| `parent` | `GetComponentInParent<T>()` | Error + disable | Non-null (`T`) |

**Edge cases:**
- `require` field with initializer → compile error: "require fields are fetched automatically and cannot have initializers."
- `require` inside a class → compile error: "'require' is only valid inside component declarations."
- `require` of a non-Component type → compile error: "'require' fields must be Unity Component types."
- Multiple `require` of the same type → allowed (different names, both fetched via `GetComponent`).

**v0 status:** **Included.**

---

## 5. Lifecycle Blocks

### Syntax

```
<lifecycle-keyword> {
    <statements>
}
```

Lifecycle blocks are **anonymous blocks** that lower directly to Unity message methods. They are only valid inside `component` declarations.

### Complete Lifecycle Table

| Moon block | Unity method | Signature | Timing |
|---|---|---|---|
| `awake` | `Awake()` | `private void` | Instance creation |
| `start` | `Start()` | `private void` | Before first frame |
| `update` | `Update()` | `private void` | Every frame |
| `fixedUpdate` | `FixedUpdate()` | `private void` | Fixed timestep |
| `lateUpdate` | `LateUpdate()` | `private void` | After all Update calls |
| `onEnable` | `OnEnable()` | `private void` | Component enabled |
| `onDisable` | `OnDisable()` | `private void` | Component disabled |
| `onDestroy` | `OnDestroy()` | `private void` | Component destroyed |
| `onTriggerEnter` | `OnTriggerEnter(Collider)` | `private void` | Trigger entered |
| `onTriggerExit` | `OnTriggerExit(Collider)` | `private void` | Trigger exited |
| `onTriggerStay` | `OnTriggerStay(Collider)` | `private void` | Trigger ongoing |
| `onCollisionEnter` | `OnCollisionEnter(Collision)` | `private void` | Collision entered |
| `onCollisionExit` | `OnCollisionExit(Collision)` | `private void` | Collision exited |
| `onCollisionStay` | `OnCollisionStay(Collision)` | `private void` | Collision ongoing |

### Parameterized Lifecycle Blocks

Collision and trigger blocks receive a parameter:

```
onTriggerEnter(other) {
    if (other.CompareTag("Player")) {
        collect()
    }
}

onCollisionEnter(collision) {
    val impact = collision.relativeVelocity.magnitude
    if (impact > threshold) {
        takeDamage(impact)
    }
}
```

The parameter name is user-chosen. The type is inferred from the lifecycle context (`Collider` for trigger events, `Collision` for collision events).

### Lowering Example

```csharp
// Moon
component Coin {
    onTriggerEnter(other) {
        if (other.CompareTag("Player")) {
            Destroy(gameObject)
        }
    }
}

// C#
public class Coin : MonoBehaviour {
    private void OnTriggerEnter(Collider other) {
        if (other.CompareTag("Player")) {
            Destroy(gameObject);
        }
    }
}
```

### Interaction with `require`/`optional`

The compiler generates a single `Awake()` method that:
1. First resolves all `require`/`optional`/`child`/`parent` fields.
2. Then executes the user's `awake` block (if any).

If any `require` field fails, the user's `awake` block does NOT execute.

```csharp
private void Awake() {
    // --- generated: require/optional resolution ---
    _rb = GetComponent<Rigidbody>();
    if (_rb == null) { /* error + disable + return */ }
    _audioSrc = GetComponent<AudioSource>(); // optional, no error

    // --- user's awake block ---
    rb.useGravity = false;
}
```

### Rules

- Each lifecycle block may appear **at most once** per component.
- Duplicate `update` → compile error: "Duplicate lifecycle block 'update'."
- Lifecycle blocks cannot have explicit return types or be called as functions.
- Lifecycle blocks inside `asset` or `class` → compile error.
- `awake` must not re-assign `require` fields → compile error: "Cannot assign to 'require' field 'rb'; it is automatically initialized."

**v0 status:** **Included.** All listed lifecycle blocks. Additional Unity messages (`OnGUI`, `OnDrawGizmos`, etc.) deferred to post-v0.

---

## 6. Type System

### 6.1 Nullable and Non-Null

**Syntax:**
```
var x: String      // non-null — cannot hold null
var y: String?     // nullable — can hold null
```

**Semantics:**
All types are **non-null by default**. Appending `?` creates the nullable variant. This is the core safety guarantee of Moon.

**Lowering:**
At the C# level, nullable reference types may or may not be emitted depending on the target C# version. The Moon compiler enforces null safety at compile time regardless of C# NRT support. For value types, `T?` lowers to `Nullable<T>`.

**Unity special case — UnityEngine.Object null:**
Unity has a custom `== null` that returns true for destroyed objects. Moon respects this:
- `require` fields are guaranteed non-null AND non-destroyed at `Awake`.
- After `Awake`, a `require` field may become "Unity-null" (destroyed). Accessing a destroyed `require` field is a runtime error (matching Unity behavior). The compiler does NOT try to track Unity object lifetime statically.
- `optional` fields are always `T?` and must be null-checked before use.

**Built-in type mappings:**

| Moon type | C# type |
|---|---|
| `Int` | `int` |
| `Float` | `float` |
| `Double` | `double` |
| `Boolean` | `bool` |
| `String` | `string` |
| `Unit` | `void` |
| `Long` | `long` |
| `Byte` | `byte` |

Value types (`Int`, `Float`, `Boolean`, etc.) are non-nullable by default. `Int?` lowers to `int?` (i.e., `Nullable<int>`).

### 6.2 Safe Call Operator (`?.`)

**Syntax:**
```
val name = enemy?.name
val health = target?.GetComponent<Health>()?.current
```

**Semantics:**
Short-circuits to `null` if the receiver is null. Return type is the nullable variant of the member's type.

**Lowering:**
```csharp
var name = enemy != null ? enemy.name : null;
// or using C# ?. directly if targeting C# 6+
var name = enemy?.name;
```

For Unity objects, the compiler emits Unity-aware null checks:
```csharp
var name = (enemy != null) ? enemy.name : null; // uses Unity's overloaded !=
```

### 6.3 Null Coalescing Operator (`??`)

**Syntax:**
```
val name = player?.name ?? "Unknown"
val source = optional audioSrc ?? GetComponent<AudioSource>()
```

**Semantics:**
If LHS is null, evaluate and return RHS. RHS must be assignable to the non-null type of LHS.

**Lowering:**
```csharp
var name = player?.name ?? "Unknown";
```

### 6.4 Non-Null Assertion (`!!`)

**Syntax:**
```
val name = nullableName!!
```

**Semantics:**
Asserts that the value is non-null. Throws `NullReferenceException` at runtime if null. Use sparingly — this is an escape hatch.

**Lowering:**
```csharp
var name = nullableName ?? throw new System.NullReferenceException("...");
// or
var name = nullableName!; // with C# NRT
```

The compiler emits a diagnostic warning: "Non-null assertion ('!!') bypasses null safety. Consider using '?.' or '??' instead."

### 6.5 Smart Casts

**Syntax:**
```
if (x is String) {
    // x is automatically String (non-null) in this scope
    print(x.length)
}

if (x != null) {
    // x is automatically non-null in this scope
    print(x.length)
}
```

**Semantics:**
After a null check or type check in a condition, the variable is narrowed to the non-null / specific type in the corresponding branch. Smart casts apply only to `val` locals and `val` fields. `var` fields cannot be smart-cast (they may change between the check and the use).

**v0 status:** Smart casts for null checks: **included**. Smart casts for type checks (`is`): **included** for simple cases only (local `val`).

### 6.6 Type Inference Boundaries

Type inference applies to:
- Local variable declarations with initializers (`val x = 5`)
- Expression-bodied function return types (`func foo() = 42` → inferred as `Int`)
- Lambda parameter types when target type is known

Type inference does NOT apply to:
- Top-level field declarations without initializers
- Function parameters (always require annotation)
- `require`/`optional`/`child`/`parent` fields (always require annotation)
- Non-expression-bodied function return types (must be annotated)
- Public API boundaries across files

**v0 status:** **Included** with the boundaries above.

---

## 7. Function Model

### 7.1 `func`

**Syntax:**
```
func <name>(<params>): <ReturnType> {
    <body>
}

func <name>(<params>): <ReturnType> = <expression>

func <name>(<params>) {
    <body>  // return type is Unit (void)
}
```

**Semantics:**
Functions are declared with `func`. Parameters are `name: Type` pairs. Return type follows the parameter list after `:`. If omitted and the body is a block, return type is `Unit`. If omitted and the body is an expression, the return type is inferred.

**Parameter features:**
- Default values: `func move(speed: Float = 1.0f)`
- Named arguments at call site: `move(speed = 2.5f)`
- No varargs in v0.

**Lowering:**
```csharp
// Moon
func calculateDamage(base: Int, multiplier: Float = 1.0f): Int {
    return (base * multiplier).toInt()
}

// C#
public int calculateDamage(int @base, float multiplier = 1.0f) {
    return (int)(@base * multiplier);
}
```

Note: Moon parameter names that are C# keywords get `@` prefix in generated code.

### 7.2 Expression-Bodied Functions

**Syntax:**
```
func isAlive(): Boolean = health > 0
func greeting() = "Hello, ${name}!"  // return type inferred as String
```

**Lowering:**
```csharp
public bool isAlive() => health > 0;
public string greeting() => $"Hello, {name}!";
```

### 7.3 Visibility

In v0, all functions in components and assets are **public** by default. A `private` modifier is available:
```
private func internalHelper(): Int = 42
```

**v0 status:** `public` (default) and `private` only. `protected`, `internal` deferred.

### 7.4 Extension Functions

**Deferred to post-v0.** Complex interactions with Unity's type system and generated code.

### 7.5 String Templates

**Syntax:**
```
val msg = "Player ${name} has ${health} HP"
val simple = "Score: $score"
```

**Lowering:** C# string interpolation (`$"..."`).

**v0 status:** **Included.**

---

## 8. Control Flow

### 8.1 `if`

**Syntax:**
```
if (<condition>) {
    <body>
}

if (<condition>) {
    <body>
} else {
    <body>
}

if (<condition>) {
    <body>
} else if (<condition>) {
    <body>
} else {
    <body>
}
```

**As expression:**
```
val max = if (a > b) a else b
```

When used as an expression, `else` is mandatory and both branches must produce compatible types.

**Lowering:**
```csharp
// statement form → C# if/else
// expression form → C# ternary
var max = (a > b) ? a : b;
```

**v0 status:** **Included.**

---

### 8.2 `when`

**Syntax:**
```
when (<subject>) {
    <pattern> -> <body>
    <pattern> -> <body>
    else -> <body>
}

// Expression form
val label = when (state) {
    State.Idle -> "Idle"
    State.Moving -> "Moving"
    else -> "Unknown"
}
```

**Semantics:**
`when` is Moon's pattern matching construct (Kotlin-style). Subjects are matched top-to-bottom. First match wins.

**Pattern types in v0:**
- Constant patterns: `42`, `"hello"`, `State.Idle`
- Type patterns: `is Enemy`
- Null pattern: `null`
- Range patterns: `in 1..10`
- Else (wildcard): `else`

**Exhaustiveness:**
- When used as an expression, `when` must be exhaustive. If the subject is an enum, all enum values must be covered OR an `else` branch must be present.
- When used as a statement, exhaustiveness is not required but the compiler emits a warning for non-exhaustive enum switches.

**Lowering:**
```csharp
// Simple enum when → C# switch expression/statement
var label = state switch {
    State.Idle => "Idle",
    State.Moving => "Moving",
    _ => "Unknown"
};

// Type pattern when → C# switch with pattern matching
switch (target) {
    case Enemy e:
        attack(e);
        break;
    case Ally a:
        heal(a);
        break;
}
```

**Edge cases:**
- `when` expression without `else` and non-exhaustive → compile error: "when expression must be exhaustive. Add an 'else' branch."
- Overlapping patterns → warning: "Pattern 'X' is unreachable because it is covered by earlier pattern 'Y'."
- `when` with no subject → allowed (acts as if-else chain):
  ```
  when {
      health < 20 -> flee()
      health < 50 -> defend()
      else -> attack()
  }
  ```

**v0 status:** **Included.**

---

### 8.3 `for`

**Syntax:**
```
for (<name> in <iterable>) {
    <body>
}

for (i in 0 until 10) { ... }       // 0..9
for (i in 0..10) { ... }            // 0..10 inclusive
for (i in 10 downTo 0) { ... }      // 10..0
for (i in 0 until 100 step 2) { ... } // 0, 2, 4, ...
```

**Semantics:**
Iterates over anything that implements `IEnumerable<T>` or has a `GetEnumerator()` method (duck-typed, matching C# `foreach` rules).

**Range expressions:**
- `a..b` — inclusive range [a, b]
- `a until b` — exclusive upper bound [a, b)
- `a downTo b` — descending range [a, b] (a >= b)
- `step N` — stride modifier

**Lowering:**
```csharp
// Moon
for (enemy in enemies) { attack(enemy) }

// C# — foreach for collections
foreach (var enemy in enemies) { attack(enemy); }

// Moon
for (i in 0 until 10) { ... }

// C# — for loop for ranges (no allocation)
for (int i = 0; i < 10; i++) { ... }
```

**Unity rationale:**
Range-based for loops lower to C-style `for` to avoid IEnumerator allocation (critical for Unity's GC-sensitive runtime).

**Edge cases:**
- `for (i in 0 until 0)` → executes zero times. Valid.
- `for` over null collection → compile error if collection is non-null type. Runtime null ref if collection is `T?` and not null-checked.
- Modifying collection during iteration → undefined (same as C#).

**v0 status:** **Included.**

---

### 8.4 `while`

**Syntax:**
```
while (<condition>) {
    <body>
}
```

**No `do-while` in v0.** (Deferred — low priority, easy to add.)

**Lowering:**
Direct mapping to C# `while`.

**`break` and `continue`:** Supported inside `for` and `while` loops.

**v0 status:** **Included.** `do-while` deferred.

---

## 9. Coroutine Model

### 9.1 Coroutine Declarations

**Syntax:**
```
coroutine <name>(<params>) {
    <body with wait statements>
}
```

**Semantics:**
A `coroutine` is a function that can suspend execution. It lowers to a Unity coroutine (returns `IEnumerator`). Coroutines may only be declared inside `component` declarations.

**Lowering:**
```csharp
// Moon
coroutine fadeOut(duration: Float) {
    var elapsed = 0.0f
    while (elapsed < duration) {
        alpha = 1.0f - (elapsed / duration)
        elapsed += Time.deltaTime
        wait()
    }
    alpha = 0.0f
}

// C#
private System.Collections.IEnumerator fadeOut(float duration) {
    float elapsed = 0f;
    while (elapsed < duration) {
        alpha = 1f - (elapsed / duration);
        elapsed += Time.deltaTime;
        yield return null;
    }
    alpha = 0f;
}
```

### 9.2 Wait Forms

| Moon | C# yield | Meaning |
|---|---|---|
| `wait()` | `yield return null` | Wait one frame |
| `wait(seconds)` | `yield return new WaitForSeconds(seconds)` | Wait N seconds |
| `wait(frames = N)` | N × `yield return null` | Wait N frames |
| `wait(until = { condition })` | `yield return new WaitUntil(() => condition)` | Wait until condition is true |
| `wait(while_ = { condition })` | `yield return new WaitWhile(() => condition)` | Wait while condition is true |
| `wait(fixedUpdate)` | `yield return new WaitForFixedUpdate()` | Wait for next FixedUpdate |
| `wait(endOfFrame)` | `yield return new WaitForEndOfFrame()` | Wait until end of frame |

**Examples:**
```
coroutine spawnWaves() {
    for (wave in 1..10) {
        spawnEnemies(wave * 5)
        wait(30.0f)  // wait 30 seconds between waves
    }
}

coroutine waitForGrounded() {
    wait(until = { isGrounded })
    land()
}
```

### 9.3 Start/Stop Behavior

**Starting a coroutine:**
```
val handle = start(fadeOut(2.0f))
```

`start()` is a built-in that lowers to `StartCoroutine()`. It returns a `CoroutineHandle` (wraps Unity's `Coroutine` return value).

**Stopping a coroutine:**
```
stop(handle)       // stop specific coroutine
stopAll()          // stop all coroutines on this component
```

**Lowering:**
```csharp
Coroutine handle = StartCoroutine(fadeOut(2.0f));
StopCoroutine(handle);
StopAllCoroutines();
```

**Rules:**
- `wait()` is only valid inside a `coroutine` body. Using it in `func` → compile error: "'wait' is only valid inside coroutine declarations."
- Coroutines cannot have return types (they implicitly return `Unit`). If you need to communicate results, use a callback parameter or set a field.
- Coroutines can call other coroutines using `wait(otherCoroutine())` to chain them.
- Coroutines can call regular `func` functions freely.

**Edge cases:**
- Calling `start()` on a destroyed component → Unity runtime error (expected, matches Unity behavior).
- `coroutine` outside a `component` → compile error: "Coroutines are only valid inside component declarations."
- Nested coroutine wait: `wait(fadeOut(1.0f))` lowers to `yield return StartCoroutine(fadeOut(1.0f))`.

**v0 status:** **Included.**

---

## 10. Unity Interop Rules

### 10.1 Calling Unity API

All `UnityEngine` types and methods are available directly. No wrapping layer.

```
// Direct Unity API usage
transform.position = Vector3(1, 2, 3)
Destroy(gameObject, 2.0f)
val hit = Physics.Raycast(origin, direction, out hitInfo, maxDistance)
Debug.Log("Hello from Moon")
```

### 10.2 `out` Parameters

C# `out` parameters are handled via the `out` keyword at the call site:

```
var hitInfo: RaycastHit
if (Physics.Raycast(ray, out hitInfo)) {
    print(hitInfo.point)
}
```

This lowers directly to C# `out` usage.

### 10.3 Generics

Moon supports calling generic Unity APIs:

```
val rb = GetComponent<Rigidbody>()
val enemies = FindObjectsOfType<Enemy>()
```

**v0 scope:** Moon code can CALL generic methods and use generic types from C#. Moon does NOT support declaring new generic types or functions in v0. Deferred.

### 10.4 Using C# Types from Moon

Any C# type visible to the Unity compilation pipeline is usable in Moon. No import statement is needed for `UnityEngine` types. For other namespaces:

```
using System.Collections.Generic

component Inventory {
    var items: List<Item> = List()
}
```

**`using` statements** appear at the top of a Moon file, before declarations.

### 10.5 Using Moon Types from C#

Generated C# classes are public and follow C# naming conventions. A C# script can reference a Moon-generated type directly:

```csharp
// In a C# file
Player player = GetComponent<Player>(); // Player was defined in Moon
player.health -= 10;
```

### 10.6 Attributes

Unity attributes can be applied using `@` syntax:

```
@Header("Movement")
var speed: Float = 5.0f

@Range(0.0f, 100.0f)
var health: Float = 100.0f

@Tooltip("The player's maximum jump height")
var jumpHeight: Float = 2.0f

@HideInInspector
var internalState: Int = 0
```

**Lowering:** Direct mapping to C# attributes (`[Header("Movement")]`, `[Range(0f, 100f)]`, etc.)

### 10.7 Constructor Syntax for Unity Types

Value types like `Vector3` use constructor syntax without `new`:

```
val pos = Vector3(1, 2, 3)
val color = Color(1, 0, 0, 1)
```

**Lowering:** `new Vector3(1, 2, 3)` — the `new` keyword is implicit in Moon.

### 10.8 Interop Limitations in v0

| Feature | Status |
|---|---|
| Calling C# methods | Included |
| Using C# types as field types | Included |
| Implementing C# interfaces | Included |
| Extending C# classes (non-component) | Included (via `class`) |
| C# calling Moon types | Included |
| Declaring generics in Moon | Deferred |
| Custom Unity Editor scripts in Moon | Deferred |
| Unity DOTS/ECS integration | Deferred |
| NativeArray/NativeSlice/Jobs | Deferred |
| Custom PropertyDrawers in Moon | Deferred |
| Unsafe code / pointers | Not planned |

**v0 status:** **Included** as specified above.

---

## 11. Lowering Rules to Backend Representation

### 11.1 Compilation Pipeline

```
Moon source (.mn)
    → Lexer → Token stream
    → Parser → AST
    → Semantic analysis (type checking, null safety, exhaustiveness)
    → Lowering → C# source (.cs)
    → Unity C# compiler → IL
    → Mono JIT / IL2CPP AOT → Native
```

### 11.2 File Mapping

Each `.mn` file produces exactly one `.cs` file in a mirror directory structure under a generated folder (e.g., `Assets/Generated/Moon/`).

```
Assets/Scripts/Player.mn → Assets/Generated/Moon/Player.cs
Assets/Scripts/Enemies/Goblin.mn → Assets/Generated/Moon/Enemies/Goblin.cs
```

### 11.3 Naming Conventions in Generated Code

| Moon | Generated C# |
|---|---|
| Component name | Class name (identical) |
| `val` field `foo` | `private T _foo; public T foo => _foo;` |
| `var` field `foo` | `public T foo;` (with `[SerializeField]`) |
| `require` field `foo` | `private T _foo; public T foo => _foo;` |
| `func foo()` | `public ReturnType foo()` |
| `private func foo()` | `private ReturnType foo()` |
| `coroutine foo()` | `private IEnumerator foo()` |
| Lifecycle block `update` | `private void Update()` |

### 11.4 Awake Method Assembly

When a component has `require`/`optional`/`child`/`parent` fields AND/OR an `awake` block, the compiler assembles a single `Awake()` method:

```csharp
private void Awake() {
    // Phase 1: Dependency resolution (generated)
    _rb = GetComponent<Rigidbody>();
    if (_rb == null) { /* error, disable, return */ }
    _audioSrc = GetComponent<AudioSource>(); // optional

    // Phase 2: User awake block (if present)
    // ... user code ...
}
```

Order within Phase 1: `require` fields are resolved in declaration order. All `require` fields are checked before any `optional`/`child`/`parent`.

### 11.5 Coroutine Lowering

```
// Moon
coroutine example() {
    wait(1.0f)
    doSomething()
    wait(until = { ready })
}

// C#
private System.Collections.IEnumerator example() {
    yield return new WaitForSeconds(1.0f);
    doSomething();
    yield return new WaitUntil(() => ready);
}
```

### 11.6 Range Loop Optimization

Range-based `for` loops are lowered to C-style `for` to avoid heap allocation:

```
// Moon
for (i in 0 until count) { process(i) }

// C#
for (int i = 0; i < count; i++) { process(i); }
```

Collection-based `for` loops use `foreach`:
```
// Moon
for (item in list) { use(item) }

// C#
foreach (var item in list) { use(item); }
```

### 11.7 Null Safety Lowering

The Moon compiler erases most null-safety information during lowering — it exists only at the Moon type-checking layer. The generated C# uses nullable reference type annotations where available (`#nullable enable`) as documentation, but correctness is guaranteed by the Moon compiler, not C# NRT.

### 11.8 Generated Code Markers

All generated files include a header:
```csharp
// <auto-generated>
// This file was generated by the Moon compiler. Do not edit manually.
// Source: Player.mn
// </auto-generated>
```

---

## 12. Explicit v0 Exclusions

The following features are **explicitly excluded from v0** with rationale:

| Feature | Rationale | Priority |
|---|---|---|
| **Generics (declarations)** | Complex type system interactions. Calling existing C# generics is sufficient for v0. | High (v1) |
| **Extension functions** | Requires method resolution changes. Can use top-level functions as workaround. | Medium (v1) |
| **Operator overloading** | Interacts with type inference. Unity types already have C# operators. | Low |
| **Abstract classes / sealed classes** | Useful but not essential for gameplay scripts. | Medium (v1) |
| **Data classes** | Kotlin-style `data class` with auto-generated `equals`/`hashCode`/`toString`. Nice-to-have. | Medium (v1) |
| **Algebraic data types (sealed enums)** | Full Kotlin-style sealed hierarchies. Complex lowering. | Medium (v1) |
| **Properties (get/set)** | `val`/`var` covers most cases. Custom getters/setters add complexity. | Medium (v1) |
| **Destructuring** | `val (x, y) = pair`. Nice but not critical. | Low |
| **Lambda literals** | Full lambda support. v0 supports lambdas only in specific positions (wait conditions, callbacks to C# APIs). | High (v1) |
| **Higher-order functions** | Requires lambda support. | High (v1) |
| **Async/await** | UniTask interop pattern. Coroutines cover v0 use cases. | Medium |
| **Annotations (custom)** | User-defined annotations. Built-in Unity attributes are sufficient. | Low |
| **Modules / packages** | Namespace management. Flat file structure with `using` is sufficient for v0. | Medium (v1) |
| **Visibility modifiers (protected, internal)** | `public` and `private` cover v0 needs. | Low |
| **do-while** | Trivial to add. Low usage. | Low |
| **try-catch-finally** | Exception handling. Can use C# try-catch via escape hatch. | High (v1) |
| **Object declarations / singletons** | Kotlin `object`. Useful for managers. | Medium (v1) |
| **Companion objects** | Kotlin `companion object`. Static member alternative. | Medium (v1) |
| **Type aliases** | `typealias`. Convenience feature. | Low |
| **Collection literals** | `listOf()`, `mapOf()`. Use C# constructors in v0. | Medium |
| **Custom Editor scripts** | `[CustomEditor]` etc. Requires editor assembly integration. | Medium |
| **DOTS/ECS** | Entirely different paradigm. Separate language feature. | Low (v2+) |
| **Multi-file components** | `partial class` equivalent. Not needed in v0. | Low |
| **Interop escape hatch (raw C#)** | Embedding C# directly. Risky for tooling. | Deferred |

---

## 13. EBNF-Style Grammar Draft (v0)

```ebnf
(* === Top Level === *)

file
    = { using_decl } , declaration ;

using_decl
    = "using" , qualified_name , NEWLINE ;

declaration
    = component_decl
    | asset_decl
    | class_decl
    | enum_decl ;

(* === Declarations === *)

component_decl
    = "component" , IDENTIFIER , [ ":" , interface_list ] , "{" , { component_member } , "}" ;

asset_decl
    = "asset" , IDENTIFIER , "{" , { asset_member } , "}" ;

class_decl
    = "class" , IDENTIFIER , [ primary_constructor ] , [ ":" , class_supers ] , "{" , { class_member } , "}" ;

enum_decl
    = "enum" , IDENTIFIER , [ "(" , enum_params , ")" ] , "{" , enum_entries , "}" ;

primary_constructor
    = "(" , param_list , ")" ;

interface_list
    = qualified_name , { "," , qualified_name } ;

class_supers
    = qualified_name , [ "(" , arg_list , ")" ] , { "," , qualified_name } ;

(* === Members === *)

component_member
    = field_decl
    | require_decl
    | optional_decl
    | child_decl
    | parent_decl
    | func_decl
    | coroutine_decl
    | lifecycle_block ;

asset_member
    = field_decl
    | func_decl ;

class_member
    = field_decl
    | func_decl ;

(* === Fields === *)

field_decl
    = { annotation } , [ "private" ] , ( "val" | "var" ) , IDENTIFIER , [ ":" , type ] , [ "=" , expression ] , NEWLINE ;

require_decl
    = "require" , IDENTIFIER , ":" , type , NEWLINE ;

optional_decl
    = "optional" , IDENTIFIER , ":" , type , NEWLINE ;

child_decl
    = "child" , IDENTIFIER , ":" , type , NEWLINE ;

parent_decl
    = "parent" , IDENTIFIER , ":" , type , NEWLINE ;

annotation
    = "@" , IDENTIFIER , [ "(" , arg_list , ")" ] ;

(* === Functions === *)

func_decl
    = { annotation } , [ "private" ] , "func" , IDENTIFIER , "(" , [ param_list ] , ")" , [ ":" , type ] , func_body ;

func_body
    = block
    | "=" , expression , NEWLINE ;

coroutine_decl
    = "coroutine" , IDENTIFIER , "(" , [ param_list ] , ")" , block ;

(* === Lifecycle === *)

lifecycle_block
    = lifecycle_keyword , [ "(" , IDENTIFIER , ")" ] , block ;

lifecycle_keyword
    = "awake" | "start" | "update" | "fixedUpdate" | "lateUpdate"
    | "onEnable" | "onDisable" | "onDestroy"
    | "onTriggerEnter" | "onTriggerExit" | "onTriggerStay"
    | "onCollisionEnter" | "onCollisionExit" | "onCollisionStay" ;

(* === Enums === *)

enum_params
    = "val" , IDENTIFIER , ":" , type , { "," , "val" , IDENTIFIER , ":" , type } ;

enum_entries
    = enum_entry , { "," , enum_entry } , [ "," ] ;

enum_entry
    = IDENTIFIER , [ "(" , arg_list , ")" ] , [ "=" , INTEGER_LITERAL ] ;

(* === Statements === *)

statement
    = val_decl
    | var_decl
    | assignment
    | expression_stmt
    | if_stmt
    | when_stmt
    | for_stmt
    | while_stmt
    | return_stmt
    | wait_stmt
    | start_stmt
    | stop_stmt
    | break_stmt
    | continue_stmt ;

val_decl
    = "val" , IDENTIFIER , [ ":" , type ] , "=" , expression , NEWLINE ;

var_decl
    = "var" , IDENTIFIER , [ ":" , type ] , [ "=" , expression ] , NEWLINE ;

assignment
    = lvalue , assign_op , expression , NEWLINE ;

assign_op
    = "=" | "+=" | "-=" | "*=" | "/=" | "%=" ;

expression_stmt
    = expression , NEWLINE ;

if_stmt
    = "if" , "(" , expression , ")" , block , [ "else" , ( block | if_stmt ) ] ;

when_stmt
    = "when" , [ "(" , expression , ")" ] , "{" , { when_branch } , "}" ;

when_branch
    = when_pattern , "->" , ( block | expression , NEWLINE ) ;

when_pattern
    = "else"
    | "null"
    | "is" , type
    | "in" , range_expr
    | expression ;

for_stmt
    = "for" , "(" , IDENTIFIER , "in" , expression , ")" , block ;

while_stmt
    = "while" , "(" , expression , ")" , block ;

return_stmt
    = "return" , [ expression ] , NEWLINE ;

wait_stmt
    = "wait" , "(" , [ wait_args ] , ")" , NEWLINE ;

wait_args
    = expression                                           (* wait(seconds) *)
    | "until" , "=" , lambda_expr                          (* wait(until = { ... }) *)
    | "while_" , "=" , lambda_expr                         (* wait(while_ = { ... }) *)
    | "frames" , "=" , expression                          (* wait(frames = N) *)
    | "fixedUpdate"                                        (* wait(fixedUpdate) *)
    | "endOfFrame"                                         (* wait(endOfFrame) *)
    ;

start_stmt
    = [ "val" , IDENTIFIER , "=" ] , "start" , "(" , call_expr , ")" , NEWLINE ;

stop_stmt
    = "stop" , "(" , expression , ")" , NEWLINE
    | "stopAll" , "(" , ")" , NEWLINE ;

break_stmt
    = "break" , NEWLINE ;

continue_stmt
    = "continue" , NEWLINE ;

(* === Expressions === *)

expression
    = or_expr , [ "?:" , expression ]                      (* null coalescing is ?? in syntax, ?:  here for EBNF clarity *)
    ;

(* Precedence from lowest to highest *)
or_expr     = and_expr , { "||" , and_expr } ;
and_expr    = equality , { "&&" , equality } ;
equality    = comparison , { ( "==" | "!=" ) , comparison } ;
comparison  = additive , { ( "<" | ">" | "<=" | ">=" | "is" | "in" | "!in" ) , additive } ;
additive    = multiplicative , { ( "+" | "-" ) , multiplicative } ;
multiplicative = unary , { ( "*" | "/" | "%" ) , unary } ;
unary       = ( "!" | "-" ) , unary | postfix ;
postfix     = primary , { postfix_op } ;

postfix_op
    = "." , IDENTIFIER                                     (* member access *)
    | "?." , IDENTIFIER                                    (* safe call *)
    | "!!"                                                 (* non-null assert *)
    | "(" , [ arg_list ] , ")"                             (* function call *)
    | "<" , type_list , ">" , "(" , [ arg_list ] , ")"     (* generic call *)
    | "[" , expression , "]"                               (* index access *)
    ;

primary
    = INTEGER_LITERAL
    | FLOAT_LITERAL
    | STRING_LITERAL
    | BOOLEAN_LITERAL
    | "null"
    | IDENTIFIER
    | "this"
    | "(" , expression , ")"
    | if_expr
    | when_expr
    | range_expr
    | lambda_expr ;

if_expr
    = "if" , "(" , expression , ")" , expression , "else" , expression ;

when_expr
    = "when" , [ "(" , expression , ")" ] , "{" , { when_branch } , "}" ;

range_expr
    = expression , ( ".." | "until" | "downTo" ) , expression , [ "step" , expression ] ;

lambda_expr
    = "{" , [ lambda_params , "->" ] , { statement } , "}" ;

lambda_params
    = IDENTIFIER , { "," , IDENTIFIER } ;

(* === Types === *)

type
    = qualified_name , [ "<" , type_list , ">" ] , [ "?" ] ;

type_list
    = type , { "," , type } ;

qualified_name
    = IDENTIFIER , { "." , IDENTIFIER } ;

(* === Parameters and Arguments === *)

param_list
    = param , { "," , param } ;

param
    = IDENTIFIER , ":" , type , [ "=" , expression ] ;

arg_list
    = arg , { "," , arg } ;

arg
    = [ IDENTIFIER , "=" ] , expression                    (* named or positional *)
    | "out" , IDENTIFIER ;                                 (* out parameter *)

(* === Blocks === *)

block
    = "{" , { statement } , "}" ;

lvalue
    = IDENTIFIER
    | postfix , "." , IDENTIFIER
    | postfix , "[" , expression , "]" ;

(* === Literals and Tokens === *)

IDENTIFIER      = /[a-zA-Z_][a-zA-Z0-9_]*/ ;
INTEGER_LITERAL = /[0-9]+[Ll]?/ ;
FLOAT_LITERAL   = /[0-9]+\.[0-9]+[fFdD]?/ ;
STRING_LITERAL  = '"' , { char | '${' , expression , '}' | '$' , IDENTIFIER } , '"' ;
BOOLEAN_LITERAL = "true" | "false" ;
NEWLINE         = /\n/ | /\r\n/ ;
```

### Reserved Keywords (v0)

```
awake       asset       break       child       class       component
continue    coroutine   downTo      else        enum        false
fixedUpdate for         func        if          in          is
lateUpdate  null        onCollisionEnter    onCollisionExit     onCollisionStay
onDestroy   onDisable   onEnable    onTriggerEnter  onTriggerExit   onTriggerStay
optional    out         override    parent      private     require
return      start       step        stop        stopAll     this
true        until       update      using       val         var
wait        when        while
```

### Soft Keywords (contextual)

```
else (in when branches)
get set (reserved for future properties)
```

---

## 14. Semantic Analysis Checklist

### Phase 1: Name Resolution

- [ ] All type references resolve to a known type (Moon-defined, C# imported, or UnityEngine).
- [ ] All `using` declarations point to valid namespaces.
- [ ] No duplicate top-level declarations within a project.
- [ ] File name matches declaration name (for components and assets).
- [ ] No forward references in field initializers (fields initialized in declaration order).

### Phase 2: Type Checking

- [ ] All expressions have a resolved type.
- [ ] Assignment LHS and RHS types are compatible.
- [ ] Function call arguments match parameter types (including named args, defaults).
- [ ] Return expression type matches declared return type.
- [ ] Binary operator operand types are compatible.
- [ ] Generic type arguments satisfy constraints (for C# generic calls).
- [ ] `out` parameters are `var` (mutable).
- [ ] Range expressions have integer operands.

### Phase 3: Null Safety

- [ ] Non-null variables are never assigned `null`.
- [ ] Nullable types are not used in positions requiring non-null without null check.
- [ ] Smart casts correctly narrow types after null/type checks.
- [ ] Safe call (`?.`) return type is nullable.
- [ ] Null coalescing (`??`) RHS type matches non-null LHS type.
- [ ] `!!` on non-null type produces warning: "Unnecessary non-null assertion."
- [ ] `require` fields are treated as non-null after `Awake`.
- [ ] `optional` fields are treated as nullable.

### Phase 4: Declaration Validation

- [ ] Components have no constructors.
- [ ] Components do not extend other components.
- [ ] Assets have no lifecycle blocks.
- [ ] Assets have no `require`/`optional`/`child`/`parent` fields.
- [ ] Classes have no lifecycle blocks.
- [ ] Classes have no `require`/`optional`/`child`/`parent` fields.
- [ ] Coroutines only appear in components.
- [ ] Lifecycle blocks only appear in components.
- [ ] Each lifecycle block appears at most once per component.
- [ ] `val` fields are assigned exactly once.
- [ ] `require` fields have no initializers.
- [ ] `require`/`child`/`parent` field types are Unity Component subtypes.
- [ ] Enum entries match parameterized enum constructor if applicable.
- [ ] Enum parameters are `val` only.
- [ ] `override` used only when implementing an interface method.

### Phase 5: Control Flow

- [ ] `when` expressions are exhaustive.
- [ ] `when` statement enum switches warn if non-exhaustive.
- [ ] Unreachable `when` patterns produce warnings.
- [ ] `break`/`continue` only inside loops.
- [ ] `wait` only inside `coroutine` body.
- [ ] All code paths in non-Unit functions return a value.
- [ ] `val` locals are definitely assigned before use.
- [ ] `var` locals are definitely assigned before use.

### Phase 6: Lowering Validation

- [ ] Generated identifiers do not collide with C# keywords (prefix with `@`).
- [ ] Generated `Awake()` assembles dependency resolution before user code.
- [ ] Range loops produce `for` (not `foreach` with allocator).
- [ ] String templates produce valid C# interpolated strings.
- [ ] Coroutine `wait` forms produce correct `yield return` variants.

---

## 15. Invalid Programs and Expected Diagnostics

### 15.1 Null Safety Violations

```
// ERROR: assigning null to non-null type
var name: String = null
```
**Diagnostic:** `E001: Cannot assign 'null' to non-null type 'String'. Use 'String?' if null is intended.`

```
// ERROR: using nullable without null check
var name: String? = getName()
print(name.length)
```
**Diagnostic:** `E002: Cannot access member 'length' on nullable type 'String?'. Use safe call '?.' or check for null first.`

```
// ERROR: returning nullable from non-null function
func getName(): String {
    return null
}
```
**Diagnostic:** `E003: Cannot return 'null' from function with non-null return type 'String'.`

---

### 15.2 Declaration Violations

```
// ERROR: constructor in component
component Player {
    constructor(name: String) { }
}
```
**Diagnostic:** `E010: Components cannot have constructors. Unity manages component lifecycle.`

```
// ERROR: component extending component
component Boss : Enemy { }
```
**Diagnostic:** `E011: Component 'Boss' cannot extend component 'Enemy'. Component inheritance is not supported. Use composition or interfaces.`

```
// ERROR: lifecycle block in asset
asset Config {
    update { }
}
```
**Diagnostic:** `E012: Lifecycle block 'update' is not valid inside an asset declaration. Lifecycle blocks are only valid in components.`

```
// ERROR: require in class
class Helper {
    require rb: Rigidbody
}
```
**Diagnostic:** `E013: 'require' fields are only valid inside component declarations.`

```
// ERROR: duplicate lifecycle block
component Player {
    update { move() }
    update { shoot() }
}
```
**Diagnostic:** `E014: Duplicate lifecycle block 'update'. Each lifecycle block may appear at most once per component.`

```
// ERROR: require with initializer
component Player {
    require rb: Rigidbody = GetComponent<Rigidbody>()
}
```
**Diagnostic:** `E015: 'require' field 'rb' cannot have an initializer. It is automatically resolved at Awake.`

```
// ERROR: require of non-Component type
component Player {
    require name: String
}
```
**Diagnostic:** `E016: 'require' field 'name' must be a Unity Component type. 'String' is not a Component.`

---

### 15.3 Type Errors

```
// ERROR: type mismatch in assignment
var health: Int = "full"
```
**Diagnostic:** `E020: Type mismatch. Expected 'Int', found 'String'.`

```
// ERROR: wrong argument type
func damage(amount: Int) { }
damage("ten")
```
**Diagnostic:** `E021: Argument type mismatch for parameter 'amount'. Expected 'Int', found 'String'.`

```
// ERROR: missing return
func getHealth(): Int {
    val h = 100
}
```
**Diagnostic:** `E022: Missing return statement. Function 'getHealth' must return 'Int' on all paths.`

---

### 15.4 Control Flow Errors

```
// ERROR: non-exhaustive when expression
val label = when (direction) {
    Direction.Up -> "Up"
    Direction.Down -> "Down"
}
```
**Diagnostic:** `E030: 'when' expression is not exhaustive. Missing branches: 'Direction.Left', 'Direction.Right'. Add an 'else' branch or cover all cases.`

```
// ERROR: break outside loop
func foo() {
    break
}
```
**Diagnostic:** `E031: 'break' is not allowed outside of a loop.`

```
// ERROR: wait outside coroutine
func foo() {
    wait(1.0f)
}
```
**Diagnostic:** `E032: 'wait' is only valid inside a coroutine declaration.`

---

### 15.5 Immutability Violations

```
// ERROR: reassigning val
val x = 5
x = 10
```
**Diagnostic:** `E040: Cannot reassign 'val' variable 'x'. Use 'var' if mutation is needed.`

```
// ERROR: reassigning require field
component Player {
    require rb: Rigidbody
    awake {
        rb = GetComponent<Rigidbody>()
    }
}
```
**Diagnostic:** `E041: Cannot assign to 'require' field 'rb'. It is automatically initialized.`

---

### 15.6 Enum Errors

```
// ERROR: empty enum
enum Empty { }
```
**Diagnostic:** `E050: Enum 'Empty' must have at least one entry.`

```
// ERROR: mutable enum param
enum Weapon(var damage: Int) {
    Sword(10)
}
```
**Diagnostic:** `E051: Enum parameters must be 'val'. Found 'var' parameter 'damage'.`

```
// ERROR: missing enum constructor args
enum Weapon(val damage: Int) {
    Sword
}
```
**Diagnostic:** `E052: Enum entry 'Sword' missing required argument 'damage: Int'.`

---

### 15.7 Coroutine Errors

```
// ERROR: coroutine outside component
class Helper {
    coroutine doThing() {
        wait(1.0f)
    }
}
```
**Diagnostic:** `E060: Coroutines are only valid inside component declarations.`

```
// ERROR: coroutine with return type
coroutine getData(): String {
    wait(1.0f)
    return "data"
}
```
**Diagnostic:** `E061: Coroutines cannot have explicit return types. They implicitly return Unit.`

---

### 15.8 File Structure Errors

```
// ERROR: multiple components in one file
component Player { }
component Enemy { }
```
**Diagnostic:** `E070: Only one top-level declaration is allowed per file. Found 'Player' and 'Enemy'.`

```
// ERROR: file name mismatch
// File: Player.mn
component Hero { }
```
**Diagnostic:** `E071: Declaration name 'Hero' does not match file name 'Player.mn'. Rename the declaration or the file.`

---

### 15.9 Warnings

```
// WARNING: unnecessary non-null assertion
val x: String = "hello"
val y = x!!
```
**Diagnostic:** `W001: Unnecessary non-null assertion '!!' on non-null type 'String'.`

```
// WARNING: non-serializable field in component
component Player {
    var callback: (() -> Unit)? = null
}
```
**Diagnostic:** `W002: Field 'callback' of type '(() -> Unit)?' is not serializable by Unity and will not appear in the Inspector.`

```
// WARNING: non-exhaustive when statement on enum
when (direction) {
    Direction.Up -> goUp()
}
```
**Diagnostic:** `W003: 'when' statement does not cover all values of 'Direction'. Missing: 'Down', 'Left', 'Right'. Consider adding an 'else' branch.`

```
// WARNING: unreachable pattern
when (x) {
    is String -> handleString()
    is String -> duplicate()
    else -> other()
}
```
**Diagnostic:** `W004: Pattern 'is String' is unreachable. It is already covered by a previous branch.`

---

## Appendix A: Complete Lowering Example

### Moon Source

```
// File: PlayerController.mn
using UnityEngine.InputSystem

component PlayerController : IDamageable {
    @Header("Movement")
    val moveSpeed: Float = 5.0f
    val jumpForce: Float = 10.0f

    @Header("Combat")
    var health: Int = 100
    var isAlive: Boolean = true

    @noserialize
    var velocity: Vector3 = Vector3.zero

    require rb: Rigidbody
    require col: CapsuleCollider
    optional audioSrc: AudioSource
    child healthBar: HealthBarUI

    awake {
        Debug.Log("PlayerController initialized on ${gameObject.name}")
    }

    start {
        health = 100
        isAlive = true
    }

    update {
        if (isAlive) {
            handleMovement()
            handleJump()
        }
    }

    fixedUpdate {
        rb.velocity = velocity
    }

    onCollisionEnter(collision) {
        if (collision.gameObject.CompareTag("Hazard")) {
            takeDamage(10)
        }
    }

    func handleMovement() {
        val h = Input.GetAxis("Horizontal")
        val v = Input.GetAxis("Vertical")
        velocity = Vector3(h, rb.velocity.y, v) * moveSpeed
    }

    func handleJump() {
        if (Input.GetButtonDown("Jump") && isGrounded()) {
            velocity = Vector3(velocity.x, jumpForce, velocity.z)
            audioSrc?.PlayOneShot(jumpSound)
        }
    }

    func isGrounded(): Boolean =
        Physics.Raycast(transform.position, Vector3.down, 1.1f)

    override func takeDamage(amount: Int) {
        health -= amount
        healthBar.setHealth(health)
        if (health <= 0) {
            isAlive = false
            start(deathSequence())
        }
    }

    coroutine deathSequence() {
        audioSrc?.PlayOneShot(deathSound)
        wait(0.5f)
        rb.isKinematic = true
        wait(2.0f)
        Destroy(gameObject)
    }
}
```

### Generated C# Output

```csharp
// <auto-generated>
// This file was generated by the Moon compiler. Do not edit manually.
// Source: PlayerController.mn
// </auto-generated>

using UnityEngine;
using UnityEngine.InputSystem;
using System.Collections;

public class PlayerController : MonoBehaviour, IDamageable
{
    [Header("Movement")]
    [SerializeField] private float _moveSpeed = 5.0f;
    public float moveSpeed => _moveSpeed;

    [SerializeField] private float _jumpForce = 10.0f;
    public float jumpForce => _jumpForce;

    [Header("Combat")]
    [SerializeField] public int health = 100;
    [SerializeField] public bool isAlive = true;

    [System.NonSerialized]
    public Vector3 velocity = Vector3.zero;

    private Rigidbody _rb;
    public Rigidbody rb => _rb;

    private CapsuleCollider _col;
    public CapsuleCollider col => _col;

    private AudioSource _audioSrc;
    public AudioSource audioSrc => _audioSrc;

    private HealthBarUI _healthBar;
    public HealthBarUI healthBar => _healthBar;

    private void Awake()
    {
        // Phase 1: Dependency resolution
        _rb = GetComponent<Rigidbody>();
        if (_rb == null)
        {
            Debug.LogError($"[PlayerController] Required component Rigidbody not found on {gameObject.name}", this);
            enabled = false;
            return;
        }

        _col = GetComponent<CapsuleCollider>();
        if (_col == null)
        {
            Debug.LogError($"[PlayerController] Required component CapsuleCollider not found on {gameObject.name}", this);
            enabled = false;
            return;
        }

        _audioSrc = GetComponent<AudioSource>();

        _healthBar = GetComponentInChildren<HealthBarUI>();
        if (_healthBar == null)
        {
            Debug.LogError($"[PlayerController] Required child component HealthBarUI not found on {gameObject.name}", this);
            enabled = false;
            return;
        }

        // Phase 2: User awake block
        Debug.Log($"PlayerController initialized on {gameObject.name}");
    }

    private void Start()
    {
        health = 100;
        isAlive = true;
    }

    private void Update()
    {
        if (isAlive)
        {
            handleMovement();
            handleJump();
        }
    }

    private void FixedUpdate()
    {
        _rb.velocity = velocity;
    }

    private void OnCollisionEnter(Collision collision)
    {
        if (collision.gameObject.CompareTag("Hazard"))
        {
            takeDamage(10);
        }
    }

    public void handleMovement()
    {
        float h = Input.GetAxis("Horizontal");
        float v = Input.GetAxis("Vertical");
        velocity = new Vector3(h, _rb.velocity.y, v) * _moveSpeed;
    }

    public void handleJump()
    {
        if (Input.GetButtonDown("Jump") && isGrounded())
        {
            velocity = new Vector3(velocity.x, _jumpForce, velocity.z);
            if (_audioSrc != null) _audioSrc.PlayOneShot(jumpSound);
        }
    }

    public bool isGrounded() =>
        Physics.Raycast(transform.position, Vector3.down, 1.1f);

    public void takeDamage(int amount)
    {
        health -= amount;
        _healthBar.setHealth(health);
        if (health <= 0)
        {
            isAlive = false;
            StartCoroutine(deathSequence());
        }
    }

    private IEnumerator deathSequence()
    {
        if (_audioSrc != null) _audioSrc.PlayOneShot(deathSound);
        yield return new WaitForSeconds(0.5f);
        _rb.isKinematic = true;
        yield return new WaitForSeconds(2.0f);
        Destroy(gameObject);
    }
}
```

---

## Appendix B: Design Decision Log

| # | Decision | Alternatives Considered | Rationale |
|---|---|---|---|
| D1 | `component` keyword instead of `class : MonoBehaviour` | Annotation-based, inheritance-based | Eliminates ceremony; makes intent unambiguous; prevents common error of forgetting base class |
| D2 | Serialized by default in components | Explicit-only serialization | Matches Unity convention; 90%+ of component fields should be serialized |
| D3 | `require` instead of explicit `GetComponent` | Property injection, attribute-based | Most common boilerplate in Unity; `require` makes dependency contracts visible at declaration |
| D4 | No component inheritance | Allow MonoBehaviour inheritance chains | Unity best practice discourages deep MonoBehaviour hierarchies; interfaces + composition preferred |
| D5 | `when` instead of `switch` | C-style switch | Kotlin-style `when` is expression-capable and safer (no fallthrough, exhaustiveness checking) |
| D6 | Coroutine as keyword, not async | async/await, UniTask-first | Unity's native coroutine model is the baseline. Async deferred to post-v0 for UniTask interop |
| D7 | `wait()` instead of `yield return` | `yield`, `suspend`, `await` | `wait` is more readable for Unity's time-based suspension model |
| D8 | Transpile to C# (not IL) | Direct IL emit, Roslyn plugin | C# output is inspectable, debuggable, and works with all Unity tooling without modification |
| D9 | No semicolons | Optional semicolons | Reduces noise; Kotlin-familiar; unambiguous with brace-based syntax |
| D10 | `val`/`var` instead of `const`/`let`/`var` | Various combinations | Directly matches Kotlin; clear mutability signal |
| D11 | `??` for null coalescing | `?:` (Kotlin), `or`, `otherwise` | `??` is familiar from C# (the target language); reduces cognitive load for Unity developers |
| D12 | One declaration per file | Multiple declarations allowed | Matches Unity's one-MonoBehaviour-per-file convention; simplifies file-to-class mapping |

---

*End of Moon Language v0 Specification*
